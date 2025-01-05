# Stage 1 - build
FROM rust:1.81 AS build
WORKDIR /app

COPY . .
RUN cargo install --path .

# Stage 2 - production
FROM debian:bookworm-slim AS final
WORKDIR /app

# Libmysqlclient-dev is required for diesel
RUN apt-get update && apt-get install -y default-libmysqlclient-dev && rm -rf /var/lib/apt/lists/*
# Compiled binary
COPY --from=build /usr/local/cargo/bin/archypix_app_back /usr/local/bin/archypix_app_back
# Static assets
COPY --from=build /app/static ./static

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=80
EXPOSE 80

CMD ["archypix_app_back"]
