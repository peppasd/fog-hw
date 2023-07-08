FROM rust:buster AS base

WORKDIR /code
RUN cargo init
COPY Cargo.toml /code/Cargo.toml
RUN cargo fetch
COPY . /code

RUN cargo build --release

FROM debian:buster-slim

EXPOSE 3000
ENV DATABASE_URL=sqlite://sqlite.db

COPY --from=base /code/target/release/cloud /cloud
COPY --from=base /code/migrations /migrations

CMD [ "/cloud" ]