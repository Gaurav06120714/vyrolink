# Build context: the core/ workspace (see render.yaml at the repository root).
FROM rust:1.99-slim-trixie AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p vl-mailbox

FROM debian:trixie-slim
COPY --from=build /src/target/release/vl-mailbox /usr/local/bin/vl-mailbox
USER nobody
EXPOSE 8080
CMD ["vl-mailbox"]
