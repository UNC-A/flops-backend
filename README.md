# Flops Backend
This repository is the monolithic server that powers the platform. Using a combination of cutting-edge technologies with a focus on efficency, correctness, and resiliance.

## Topology
### Axum Websocket
Axum WS provides the primary connectivity between the backend and client, WS was chosen for its low latency and simplistic session based design. Axum itself is a very well designed toolkit made by the Tokio team.
### MongoDB
MongoDB is designed with horizontal scaling in mind, since this application intends on runing 100s of threads at once their focus on concurrency is critical. Additionally MonogoDB's MQL schema allows for quick development and clever querying.
### In Memory Database (volatile DB)
For tasks that need to run several 1000 queries per second we've decided to use an atomic reference counter with a write-lock to create a fast database. This DB handles data relevant to the WS connection (online users, pending messages, etc).
### Rust
The features and philosophy of the Rust language closely align with our software needs. The design delegates many potenial runtime errors to compile checks, that and other features enables us to write more reliable software.
