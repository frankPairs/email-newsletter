# BUILDER STAGE
# ---------- INSTALLATION DEPENDENCIES STAGE ----------
FROM lukemathwalker/cargo-chef:latest-rust-1.66.1 AS chef

# Switch our working directory (inside the docker container) to 'app' folder. It will create the folder if does not exist. 
WORKDIR /app

# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y

# ---------- CARGO CHEF PREPARATION STAGE ----------
FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

# ---------- BUILDER STAGE ----------
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
# cargo chef cook builds our project dependencies, not our application!!
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
# It allows to run sqlx compilation verifications without having to connect with a database. This is possible just after executing the command
# ```cargo sqlx prepare -- --lib``` which creates a sqlx-data.json file.
ENV SQLX_OFFLINE true

RUN cargo build --release --bin email_newsletter

# ---------- RUNTIME STAGE ----------
# It uses a small container as we only need to execute the binary generated by the 'builder step'
FROM debian:bullseye-slim AS runtime

WORKDIR /app

RUN apt-get update -y \ 
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/email_newsletter email_newsletter
COPY config config

ENV APP_ENVIRONMENT production

ENTRYPOINT [ "./email_newsletter" ]