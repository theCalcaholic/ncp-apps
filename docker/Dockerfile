FROM alpine:latest as ncp-src
RUN apk add git
RUN git clone https://github.com/nextcloud/nextcloudpi

#FROM ubuntu:latest as cargo-builder
#RUN apt-get update && apt-get install curl build-essential ca-certificates file xutils-dev nmap -y
#RUN mkdir /rust
#WORKDIR /rust
#RUN curl https://sh.rustup.rs -s >> rustup.sh
#RUN chmod 755 /rust/rustup.sh
#RUN ./rustup.sh -y
#ENV PATH=/root/.cargo/bin:$PATH
#RUN rustup default 1.72.1
#RUN mkdir /app
#WORKDIR /app
#COPY Cargo.toml /app/
#RUN cargo build
#ADD . /app
#RUN cargo build

FROM alpine:latest as ncp-apps
RUN mkdir -p /ncp/etc
COPY --from=ncp-src /nextcloudpi/bin /ncp/bin
COPY --from=ncp-src /nextcloudpi/etc/ncp-templates /ncp/etc/ncp-templates
COPY --from=ncp-src /nextcloudpi/etc/library.sh /ncp/etc/library.sh
COPY ./target/release/ncp /usr/local/bin/
#COPY --from=cargo-builder /app/target/debug/ncp /usr/local/bin

ENTRYPOINT ["ncp"]
