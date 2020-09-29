FROM rust:1.46

RUN useradd discord
RUN mkdir -p /workspace && mkdir -p /workspace/bin
WORKDIR /workspace
USER discord
RUN cargo install --version=0.1.0-beta.1 sqlx-cli --no-default-features --features postgres
COPY --chown=discord:discord ./data ./data
COPY --chown=discord:discord ./migrations ./migrations
COPY --chown=discord:discord ./target/release/erebor-record-keeper ./target/release/fetch_scenarios ./target/release/load_challenges ./bin/
CMD ["/workspace/bin/erebor-record-keeper"]
