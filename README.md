# Archypix App Back

The backend of the Archypix app.
It is an HTTP Rocket api.

# Build & run

## Required services

- A PostgreSQL database.
- A S3 storage, such as [MinIO](https://min.io/).
- A SMTP server.

You can use run PostgreSQL and MinIO with Docker compose:

```bash
docker compose up archypix-app-postgres archypix-app-minio
```

## Configure the app

Configure the app and how to access the services in the `.env` file. (see `.env.example`).

## Run with dependant system libraries installed (gexiv2, libpq, imagemagick)
```bash
cargo run
```

## Run with nix

```bash
nix develop
cargo run
```

## Run with docker

The docker image is configured to run in production, and is not recommended for development.

```bash
docker compose up archypix-app-back .
```

# Licence
This code is licensed under the [Elastic License 2.0 (ELv2)](https://www.elastic.co/licensing/elastic-license)
