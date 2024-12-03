mod diff; 
mod notepad;


use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{
        Hash, Hasher
    },
    time::Duration
};
use diff::{Diff, MessageBuf, Operation};
use futures::stream::StreamExt;
use libp2p::{
    gossipsub, mdns, noise, tcp, yamux,
    swarm::{
        NetworkBehaviour, SwarmEvent
    }
};
use notepad::Notepad;
use tokio::{
    io, select,
    io::AsyncBufReadExt
};
use tracing_subscriber::EnvFilter;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(), 
            noise::Config::new, 
            yamux::Config::default
        )?
        .with_quic()
        .with_behaviour(|key| {
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();

                message.sequence_number.hash(&mut s);
                message.data.hash(&mut s);

                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()), 
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

            Ok(MyBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let mut current_topic = gossipsub::IdentTopic::new("test-net");

    swarm.behaviour_mut().gossipsub.subscribe(&current_topic)?;

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");

    let mut current_notepad = Notepad::default();
    current_notepad.text = "hello world".to_string();

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                let mut parts = line.splitn(3, ':');
                let op = parts.next().unwrap();
                let value = parts.next();
                let char = parts.next();
                
                let mut message = MessageBuf::default();

                match op {
                    "see" => {
                        println!("current notepad: {current_notepad:?}");
                    },
                    "swi" => {
                        if let Some(value) = value {
                            swarm.behaviour_mut().gossipsub.unsubscribe(&current_topic)?;
                            current_topic = gossipsub::IdentTopic::new(value);
                            swarm.behaviour_mut().gossipsub.subscribe(&current_topic)?;
                            println!("Switching to room: `{:?}`", value);        
                        } else {
                            println!("Expected format `swi:value`");
                        } 
                    },
                    "ins" => {
                        if let Some(index) = value {
                            if let Some(char) = char {
                                // cannot handle escaped i.e '\n'
                                if char.len() == 1 {
                                    message.messages.push( 
                                        Diff { 
                                            opcode: Operation::Ins, 
                                            operand: Some( char
                                                .chars()
                                                .nth(0)
                                                .unwrap()
                                            ), 
                                            index: index
                                                .parse::<u8>()
                                                .expect("`index` failed to parse to `u8`") 
                                        }
                                    );
                                } else {
                                    println!("Expects char to be a single character")
                                }
                            } else {
                                println!("Expected format `ins:index:char`");
                            }
                        } else {
                            println!("Expected format `ins:index:char`");
                        }
                    },
                    "del" => {
                        if let Some(index) = value {
                            message.messages.push(
                                Diff {
                                    opcode: Operation::Del,
                                    operand: None,
                                    index: index
                                        .parse::<u8>()
                                        .expect("`index` failed to parse to `u8`")
                                }
                            )
                        } else {
                            println!("Expected format `del:index`");
                        }
                    },
                    "rep" => {
                        if let Some(index) = value {
                            if let Some(char) = char {
                                // cannot handle escaped i.e '\n'
                                if char.len() == 1 {
                                    message.messages.push( 
                                        Diff { 
                                            opcode: Operation::Rep, 
                                            operand: Some( char
                                                .chars()
                                                .nth(0)
                                                .unwrap()
                                            ), 
                                            index: index
                                                .parse::<u8>()
                                                .expect("`index` failed to parse to `u8`") 
                                        }
                                    );
                                } else {
                                    println!("Expects char to be a single character")
                                }
                            } else {
                                println!("Expected format `rep:index:char`");
                            }
                        } else {
                            println!("Expected format `rep:index:char`");
                        }
                    }
                    _ => {
                        println!("Unknown opcode: {op:?}");
                    },
                }

                current_notepad.apply_message_buf(&message);

                let message_bytes: Vec<u8> = message.into();

                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(current_topic.clone(), message_bytes) {
                        match e {
                            gossipsub::PublishError::InsufficientPeers => {},
                            _ => println!("Publish error: {e:?}")
                        }
                }

            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: _peer_id,
                    message_id: _id,
                    message,
                })) => {
                    let msg: MessageBuf = message.data.into();
                    println!("Current notepad: {current_notepad:?}");
                    current_notepad.apply_message_buf(&msg);
                    println!("Updated notepad: {current_notepad:?}");
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
}

