use std::{collections::HashMap, time::Duration};
use tracing::{error, warn};

use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{self, Instant},
};

use crate::actors::{Actor, ActorType, ControlMessage};

pub struct Supervisor {
    actor_factories: HashMap<ActorType, Box<dyn Fn() -> Box<dyn Actor> + Send + Sync>>,
    pulses: HashMap<ActorType, Instant>,
    handles: HashMap<ActorType, JoinHandle<()>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            actor_factories: HashMap::new(),
            pulses: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    pub fn register_actor(
        &mut self,
        actor_type: ActorType,
        factory: Box<dyn Fn() -> Box<dyn Actor> + Send + Sync>,
    ) {
        self.actor_factories.insert(actor_type, factory);
    }

    pub async fn start(&mut self) {
        let mut check_interval = time::interval(Duration::from_secs(1));
        let timeout_duration = Duration::from_secs(3);

        let (supervisor_tx, mut supervisor_rx) = mpsc::channel::<ControlMessage>(512);

        let actors: Vec<ActorType> = self
            .actor_factories
            .iter()
            .map(|(actor, _)| actor.clone())
            .collect();

        actors.into_iter().for_each(|actor| {
            self.spawn_actor(actor, supervisor_tx.clone());
        });

        loop {
            tokio::select! {
                Some(msg) = supervisor_rx.recv() => {
                    match msg {
                        ControlMessage::Heartbeat(actor_type) => {
                            self.pulses.insert(actor_type, Instant::now());
                        }
                        ControlMessage::Shutdown(actor_type) => {
                            warn!("{:?} is shutting down gracefully.", actor_type);
                            self.pulses.remove(&actor_type);
                            if let Some(handle) = self.handles.remove(&actor_type) {
                                handle.abort();
                            }
                        },
                        ControlMessage::Error(actor_type, error_msg) => {
                            error!("Actor {:?} reported error: {}", actor_type, error_msg);
                            self.pulses.insert(actor_type, Instant::now());
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
                            self.handles[key].abort();
                        }
                    }

                    dead_actors.into_iter().for_each(|i| {
                        self.spawn_actor(i, supervisor_tx.clone());
                    });
                }
            }
        }
    }

    fn spawn_actor(&mut self, actor_type: ActorType, tx: mpsc::Sender<ControlMessage>) {
        let mut new_actor = self.actor_factories[&actor_type]();
        let new_actor_handle = tokio::spawn(async move {
            if let Err(e) = new_actor.run(tx).await {
                error!("Actor {:?} crashed: {}", &actor_type, e);
            }
        });
        self.handles.insert(actor_type.clone(), new_actor_handle);
        self.pulses.insert(actor_type, Instant::now());
    }
}
