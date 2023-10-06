use std::io::{Read, stdout, Write};
use std::time::Duration;
use bollard::Docker;
use bollard::container;
use bollard::container::{AttachContainerOptions, AttachContainerResults, RemoveContainerOptions};
use bollard::image::CreateImageOptions;
use clap::parser::ValueSource::DefaultValue;
use termion::async_stdin;
use termion::raw::IntoRawMode;
use futures_util::{StreamExt, TryStreamExt};
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use tokio::task::spawn;

pub async fn test() -> Result<(), anyhow::Error> {
    let docker = Docker::connect_with_socket_defaults()?;
    let version = docker.version().await?;
    println!("{:?}", version);
    // println!("Creating image...");
    // docker
    //     .create_image(
    //         Some(CreateImageOptions {
    //             from_image: "alpine:latest",
    //             ..Default::default()
    //         }),
    //         None,
    //         None)
    //     .try_collect::<Vec<_>>()
    //     .await?;
    println!("Creating container...");
    let container_config = container::Config {
        image: Some("alpine:latest"),
        working_dir: Some("/ncp"),
        cmd: Some(vec!("echo", "hello world")),
        // tty: Some(true),
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        open_stdin: Some(true),
        ..Default::default()
    };
    let container_id = docker
        .create_container::<&str, &str>(None, container_config)
        .await?
        .id;
    println!("Starting container...");
    docker.start_container::<String>(&container_id, None).await?;
    println!("Attaching to container...");
    {
        let AttachContainerResults {
            mut output,
            mut input
        } = docker.attach_container(
            &container_id,
            Some(AttachContainerOptions::<String> {
                stdout: Some(true),
                stderr: Some(true),
                stdin: Some(true),
                stream: Some(true),
                ..Default::default()
            }),
        ).await?;


        println!("Attaching to stdin...");
        spawn(async move {
            let mut stdin = async_stdin().bytes();
            loop {
                if let Some(Ok(byte)) = stdin.next() {
                    input.write(&[byte]).await.ok();
                } else {
                    sleep(Duration::from_nanos(10)).await;
                }
            }
        });
        println!("Attaching to stdout...");
        let stdout = stdout();
        let mut stdout = stdout.lock().into_raw_mode()?;
        while let Some(Ok(output)) = output.next().await {
            stdout.write_all(output.into_bytes().as_ref())?;
            stdout.flush()?;
        }
    }
    println!("Removing container...");
    docker.remove_container(
        &container_id,
        Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        }),
    )
        .await?;
    println!("Done");
    Ok(())
}
