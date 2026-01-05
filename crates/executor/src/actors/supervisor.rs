use std::{collections::HashMap, time::Duration};
use tracing::{error, info, warn};

use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{self, Instant},
};
use uuid::Uuid;

use crate::actors::{Actor, ActorType, ControlMessage};

pub struct Supervisor {
    actor_factories: HashMap<ActorType, Box<dyn Fn() -> Box<dyn Actor> + Send + Sync>>,
    pulses: HashMap<Uuid, Instant>,
    handles: HashMap<Uuid, JoinHandle<()>>,
    actor_types: HashMap<Uuid, ActorType>,
    tx: mpsc::Sender<ControlMessage>,
    rx: Option<mpsc::Receiver<ControlMessage>>,
}

impl Supervisor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(512);
        Self {
            actor_factories: HashMap::new(),
            pulses: HashMap::new(),
            handles: HashMap::new(),
            actor_types: HashMap::new(),
            tx,
            rx: Some(rx),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<ControlMessage> {
        self.tx.clone()
    }

    pub fn register_actor(
        &mut self,
        actor_type: ActorType,
        factory: Box<dyn Fn() -> Box<dyn Actor> + Send + Sync>,
    ) {
        self.actor_factories.insert(actor_type, factory);
    }

    fn initialize_actors(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) {
        let actors: Vec<(ActorType, Box<dyn Actor>)> = self
            .actor_factories
            .iter()
            .map(|f| (f.0.clone(), f.1()))
            .collect();

        actors.into_iter().for_each(|(actor, factory)| {
            self.spawn_actor(factory, actor, supervisor_tx.clone());
        });
    }

    pub async fn start(&mut self) {
        let mut check_interval = time::interval(Duration::from_secs(1));
        let timeout_duration = Duration::from_secs(3);

        let supervisor_tx = self.tx.clone();
        let mut supervisor_rx = self.rx.take().expect("Supervisor started twice");
        self.initialize_actors(supervisor_tx.clone());

        loop {
            tokio::select! {
                Some(msg) = supervisor_rx.recv() => {
                    match msg {
                        ControlMessage::Spawn(actor) => {
                            info!("Spawning dynamic actor: {:?}", actor.id());
                            let tx = supervisor_tx.clone();
                            self.spawn_actor(actor, ActorType::Dynamic, tx);
                        },
                        ControlMessage::Heartbeat(actor_id) => {
                            self.pulses.insert(actor_id, Instant::now());
                        }
                        ControlMessage::Shutdown(actor_id) => {
                            warn!("{:?} is shutting down gracefully.", actor_id);
                            self.pulses.remove(&actor_id);
                            self.actor_types.remove(&actor_id);
                            if let Some(handle) = self.handles.remove(&actor_id) {
                                handle.abort();
                            }
                        },
                        ControlMessage::Error(actor_id, error_msg) => {
                            error!("Actor {:?} reported error: {}", actor_id, error_msg);
                            self.pulses.insert(actor_id, Instant::now());
                        },
                    }
                }

                _ = check_interval.tick() => {
                    let dead_timeout = Instant::now() - timeout_duration;

                    let mut dead_actors = Vec::new();

                    for (key, &value) in self.pulses.iter() {
                        if value < dead_timeout {
                            warn!("{:?} is unresponsive!", key);
                            dead_actors.push(key.clone());
                            if let Some(handle) = self.handles.get(key) {
                                handle.abort();
                            }
                        }
                    }

                    dead_actors.into_iter().for_each(|invalid_id| {
                        let actor_t = self.actor_types[&invalid_id];
                        if self.actor_factories.contains_key(&actor_t) {
                            info!("Restarting actor type {:?} (old id: {:?}", actor_t, invalid_id);
                            let new_actor = self.actor_factories[&actor_t]();
                            self.spawn_actor(new_actor, actor_t, supervisor_tx.clone());
                        } else {
                            warn!("Dynamic actor {:?} died and will not be restarted.", invalid_id);
                        }
                        self.pulses.remove(&invalid_id);
                        self.handles.remove(&invalid_id);
                        self.actor_types.remove(&invalid_id);
                    });
                }
            }
        }
    }

    fn spawn_actor(
        &mut self,
        mut actor: Box<dyn Actor>,
        actor_type: ActorType,
        tx: mpsc::Sender<ControlMessage>,
    ) {
        let actor_id = actor.id();
        let new_actor_handle = tokio::spawn(async move {
            if let Err(e) = actor.run(tx).await {
                error!("Actor {:?} crashed: {}", &actor_type, e);
            }
        });
        self.actor_types.insert(actor_id, actor_type);
        self.handles.insert(actor_id, new_actor_handle);
        self.pulses.insert(actor_id, Instant::now());
    }
}
