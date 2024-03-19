# Build Stage
FROM  rust:latest
USER 0:0
WORKDIR /home/rust/src

# Install build requirements

RUN apt-get update && \
    apt-get install -y \
    make \
    pkg-config \
    libssl-dev \
    mold

# Build all dependencies
COPY src ./src
COPY Cargo.* ./
COPY *.json ./
COPY .env ./.env
RUN ls
RUN mold --run cargo b -r
