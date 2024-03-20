# get base image
FROM  rust:latest
USER 0:0
WORKDIR /home/rust/src


# install dependancies
# mold links slightly faster so we use it
RUN apt-get update && \
    apt-get install -y \
    make \
    pkg-config \
    libssl-dev \
    mold

# copy files
COPY src ./src
COPY Cargo.* ./
COPY *.json ./
COPY .env ./.env
RUN ls

# compile server
RUN mold --run cargo b -r
