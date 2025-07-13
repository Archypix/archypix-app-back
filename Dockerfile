# Stage 1 - build
FROM nixos/nix:latest AS build
WORKDIR /app

COPY . .
RUN rm .env*

ENV NIXPKGS_ALLOW_UNFREE=1
RUN nix \
    --extra-experimental-features "nix-command flakes" \
    build

# Copy the Nix store closure into a directory. The Nix store closure is the
# entire set of Nix store values that we need for our build.
RUN mkdir ./nix-store-closure
RUN cp -R $(nix-store -qR result/) ./nix-store-closure


FROM scratch AS prod
WORKDIR /app

# /nix/store
COPY --from=build /app/nix-store-closure /nix/store
# Compiled binary
COPY --from=build /app/result/bin .
# Static assets
COPY --from=build /app/static ./static

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=80
EXPOSE 80

CMD ["/app/archypix_app_back"]
