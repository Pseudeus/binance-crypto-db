podman run --rm -it \
 --device=/dev/kfd \
   --device=/dev/dri \
   --security-opt=label=disable \
   -v $(pwd)/training:/app/data:z \
   -v $(pwd)/models:/app/models:z \
   -e COLLECTED_DB_PATH=/app/data/db.sql.zst \
   gemini-trainer
