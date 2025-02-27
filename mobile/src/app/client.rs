use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpStream},
    sync::Mutex,
};

use rodio::Sink;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::select;

use crate::app::backend::{
    decrypt_aes256, display_error_message, write_audio, ClientMessage, ClientMessageType,
    ConnectionState, MessageReaction, PlaybackCursor, Reaction, ServerReplyType, ServerSync,
    ServerVoipReply,
};

use crate::app::backend::{Application, ServerMessageType};

/// This is the byte lenght of the uuid's text representation (utf8)
pub const UUID_STRING_BYTE_LENGHT: usize = 36;

/// Hash byte offset
/// This value is the end of the uuid string bytes, the start is ```IDENTIFICATOR_BYTE_OFFSET```
pub const UUID_BYTE_OFFSET: usize = 64 + UUID_STRING_BYTE_LENGHT;

/// Image byte offset
/// This value is the end of the hash bytes, the start is ```UUID_BYTE_OFFSET```
pub const HASH_BYTE_OFFSET: usize = 64 + UUID_BYTE_OFFSET;

/// Identificator byte offset
/// This value is the start of the identificator bytes, the end is the end of the message itself
pub const IDENTIFICATOR_BYTE_OFFSET: usize = 64;

/// This is the byte length of the uuid's text representation (utf8)
pub const UUID_STRING_BYTE_LENGTH: usize = 36;

use super::backend::{fetch_incoming_message_length, get_image_header};
pub const VOIP_PACKET_BUFFER_LENGTH_MS: usize = 35;

use std::io::{BufReader, Cursor};

use crate::app::backend::{decrypt_aes256_bytes, MessageBuffer, UdpMessageType};

/// Sends connection request to the specified server handle, returns the server's response, this function does not create a new thread, and may block
pub async fn connect_to_server(
    mut connection: TcpStream,
    message: ClientMessage,
) -> anyhow::Result<(String, TcpStream)>
{
    let message_as_string = message.struct_into_string();

    let message_bytes = message_as_string.as_bytes();

    //Send message length to server
    connection
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message to server
    connection.write_all(message_bytes).await?;

    //Read the server reply length
    //blocks here for unknown reason
    let msg_len = fetch_incoming_message_length(&mut connection).await?;

    //Create buffer with said length
    let mut msg_buffer = vec![0; msg_len as usize];

    //Read the server reply
    connection.read_exact(&mut msg_buffer).await?;

    Ok((String::from_utf8(msg_buffer)?, connection))
}

pub struct ServerReply
{
    pub reader: Arc<Mutex<OwnedReadHalf>>,
}

impl ServerReply
{
    pub async fn wait_for_response(&self) -> anyhow::Result<String>
    {
        let reader = &mut *self.reader.lock().await;

        // Read the server reply length
        let msg_len = fetch_incoming_message_length(reader).await?;

        //Create buffer with said length
        let mut msg_buffer = vec![0; msg_len as usize];

        //Read the server reply
        reader.read_exact(&mut msg_buffer).await?;

        Ok(String::from_utf8(msg_buffer)?)
    }

    pub fn new(reader: Arc<Mutex<OwnedReadHalf>>) -> Self
    {
        Self { reader }
    }
}

impl Application
{
    ///This function is used to send voice recording in a voip connection, this function spawns a thread which record 35ms of your voice then sends it to the linked voip destination
    // pub fn client_voip_thread(&mut self, ctx: &egui::Context)
    // {
    //     if let Some(voip) = self.client_ui.voip.clone() {
    //         let uuid = self.opened_user_information.uuid.clone();
    //         let destination = self.client_ui.send_on_ip.clone();
    //         let decryption_key = self.client_connection.client_secret.clone();
    //         let cancel_token = self.voip_shutdown_token.clone();
    //         let cancel_token_child = cancel_token.child_token();
    //         let uuid_clone = uuid.clone();
    //         let decryption_key_clone = decryption_key.clone();
    //         let voip_clone = voip.clone();
    //         let camera_handle = voip_clone.camera_handle.clone();
    //         let voice_recording_shutdown = self.voip_video_shutdown_token.clone();

    //         self.voip_thread.get_or_insert_with(|| {
    //             let receiver_socket_part = voip.socket.clone();
    //             let microphone_percentage = self.client_ui.microphone_volume.clone();

    //             let (tx, rx) = mpsc::channel::<()>();

    //             self.record_audio_interrupter = tx;

    //             let enable_microphone = voip.enable_microphone.clone();

    //             //Sender thread
    //             tokio::spawn(async move {
    //                 //This variable is notified when the Mutex is set to true, when the audio_buffer length reaches ```VOIP_PACKET_BUFFER_LENGTH``` and is reset when the packet is sent
    //                 let voip_audio_buffer: Arc<std::sync::Mutex<VecDeque<f32>>> = Arc::new(std::sync::Mutex::new(VecDeque::new()));

    //                 //Connect socket to destination
    //                 voip.socket.connect(destination).await.unwrap();

    //                 //Start audio recorder
    //                 let recording_handle = record_audio_with_interrupt(rx, *microphone_percentage.lock().unwrap(), voip_audio_buffer.clone(), enable_microphone.clone()).unwrap();

    //                 //We can just send it because we have already set the default destination address
    //                 loop {
    //                     select! {
    //                         //Wait until we should send the buffer
    //                         //Record 35ms of audio, send it to the server
    //                         _ = tokio::time::sleep(Duration::from_millis(VOIP_PACKET_BUFFER_LENGTH_MS as u64)) => {
    //                                 //We create this scope to tell the compiler the recording handle wont be sent across any awaits
    //                                 let playbackable_audio: Vec<u8> = {
    //                                     //Lock handle
    //                                     let mut recording_handle = recording_handle.lock().unwrap();
    //                                     //Create wav bytes
    //                                     let playbackable_audio: Vec<u8> = create_wav_file(
    //                                         recording_handle.clone().into()
    //                                     );
    //                                     //Clear out buffer, make the capacity remain (We created this VecDeque with said default capacity)
    //                                     recording_handle.clear();
    //                                     //Return wav bytes
    //                                     playbackable_audio
    //                                 };
    //                                 //Create audio chunks
    //                                 let audio_chunks = playbackable_audio.chunks(30000);
    //                                 //Avoid sending too much data (If there is more recorded we just iterate over the chunks and not send them at once)
    //                                 for chunk in audio_chunks {
    //                                     voip.send_audio(uuid.clone(), chunk.to_vec(), &decryption_key).await.unwrap();
    //                                 }
    //                         },
    //                         _ = cancel_token.cancelled() => {
    //                             //Exit thread
    //                             break;
    //                         },
    //                     };
    //                 }
    //             });

    //             //Clone ctx
    //             let ctx = ctx.clone();

    //             //Create sink
    //             let sink = Arc::new(rodio::Sink::try_new(&self.client_ui.audio_playback.stream_handle).unwrap());
    //             let decryption_key = self.client_connection.client_secret.clone();

    //             //Receiver thread
    //             tokio::spawn(async move {
    //                 let ctx_clone = ctx.clone();

    //                 //Create image buffer
    //                 let image_buffer: MessageBuffer = Arc::new(DashMap::new());

    //                 //Listen on socket, play audio
    //                 loop {
    //                     select! {
    //                         _ = cancel_token_child.cancelled() => {
    //                             //Break out of the listener loop
    //                             break;
    //                         },

    //                         //Receive bytes
    //                         _received_bytes_count = async {
    //                             match receive_server_relay(receiver_socket_part.clone(), &decryption_key, sink.clone(), image_buffer.clone(), &ctx_clone).await {
    //                                 Ok(_) => (),
    //                                 Err(err) => {
    //                                     tracing::error!("{}", err);
    //                                 },
    //                             }
    //                         } => {}
    //                     }
    //                 }
    //             });
    //         });

    //         if let Ok(handle) = camera_handle.try_lock() {
    //             if handle.is_none() {
    //                 return;
    //             }
    //         }

    //         self.voip_video_thread.get_or_insert_with(|| {
    //             //Create image sender thread
    //             tokio::spawn(async move {
    //                 loop {
    //                     select! {
    //                         //Lock camera handle
    //                         mut camera_handle = camera_handle.lock() => {
    //                             //Get image bytes from the cameras
    //                             match camera_handle.as_mut() {
    //                                 Some(handle) => {
    //                                     //Create buffer for image
    //                                     let mut buffer = BufWriter::new(Cursor::new(Vec::new()));
    //                                     //Get camera frame
    //                                     let (camera_bytes, size) = handle.get_frame().unwrap_or_default();

    //                                     //Convert raw image bytes to jpeg
    //                                     image::write_buffer_with_format(&mut buffer, &camera_bytes, size.width as u32, size.height as u32, image::ColorType::Rgb8, ImageOutputFormat::Jpeg(70)).unwrap();

    //                                     //Send image
    //                                     voip_clone.send_image(uuid_clone.clone(), &buffer.into_inner().unwrap().into_inner(), &decryption_key_clone).await.unwrap();
    //                                 },
    //                                 None => {
    //                                     //... camera handle has been removed
    //                                     break;
    //                                 },
    //                             }
    //                         }
    //                         _ = voice_recording_shutdown.cancelled() => {
    //                             println!("exit thread");
    //                             //Exit thread
    //                             break;
    //                         },
    //                     }
    //                 }
    //             });
    //         });
    //     }
    // }

    ///This functions is used for clients to receive messages from the server (this doesnt not check validity of the order of the messages, although this may not be needed as tcp takes care of this)
    pub fn client_recv(&mut self, ctx: &egui::Context)
    {
        //This should only run when the connection is valid
        if let ConnectionState::Connected(connection_pair) = self.client_connection.state.clone() {
            self.server_sender_thread.get_or_insert_with(|| {
                //Clone so we can move it into the closure
                let sender = self.server_output_sender.clone();

                //Clone the reader so we can move it in the closure
                let reader = connection_pair.reader.clone();

                //Clone the sender so that 2 threads can each get a sender
                let sender_clone = sender.clone();

                //We clone ctx, so we can call request_repaint from inside the thread
                let context_clone = ctx.clone();

                //Thread cancellation token
                let shutdown_token = self.autosync_shutdown_token.child_token();

                //We have to clone for the 2nd thread
                let shutdown_token_clone = shutdown_token.clone();

                let toasts = self.toasts.clone();

                //Spawn server reader thread
                tokio::spawn(async move {
                    loop {
                        let server_reply_handle = &ServerReply {
                            reader: reader.clone(),
                        };

                        select! {
                        //Receive input from main thread to shutdown
                            _ = shutdown_token.cancelled() => {
                                break;
                            },

                            reply = ServerReply::wait_for_response(server_reply_handle) => {
                                match reply {
                                    //If we have a response from the server
                                    Ok(response) => {
                                        //Check for special cases like server disconnecting
                                        if response == "Server disconnecting from client." {
                                            break;
                                        }

                                        //Request repaint
                                        context_clone.request_repaint();
                                        //Send to receiver
                                        sender_clone.send(Some(response)).expect("Error occurred when trying to send message, after receiving message from client");
                                    },
                                    Err(err) => {
                                        tracing::error!("{}", err);

                                        eprintln!("client.rs\nError occurred when the client tried to receive a message from the server: {err}");
                                        eprintln!("Early end of file error is a normal occurrence after disconnecting");
                                        //Avoid panicking when trying to display a Notification
                                        //This is very rare but can still happen 
                                        display_error_message(err, toasts);

                                        //Error appeared, after this the tread quits, so there arent an inf amount of threads running
                                        let _ = sender_clone.send(None);

                                        break;
                                    },
                                }
                            }
                        }
                    }
                });

                //Init sync message
                let mut message = ClientMessage::construct_sync_msg(
                    &self.client_connection.password,
                    &self.login_username,
                    &self.opened_user_information.uuid,
                    //Send how many messages we have, the server will compare it to its list, and then send the missing messages, reducing traffic
                    self.client_ui.incoming_messages.message_list.len(),
                    Some(*self.client_ui.last_seen_msg_index.lock().unwrap()),
                );

                let last_seen_message_index = self.client_ui.last_seen_msg_index.clone();

                //Spawn server syncer thread
                tokio::spawn(async move {
                    loop {
                        //This patter match will always return true, the message were trying to pattern match is constructed above 
                        //We should update the message for syncing, so we will provide the latest info to the server
                        if let ClientMessageType::SyncMessage(inner) = &mut message.message_type {
                            tokio::time::sleep(Duration::from_secs(2)).await;

                            //We should only check for the value after sleep
                            if shutdown_token_clone.is_cancelled() {
                                break;
                            }

                            let index = *last_seen_message_index.lock().unwrap();

                            if inner.last_seen_message_index < Some(index) {
                                inner.last_seen_message_index = Some(index);

                                //We only send a sync packet if we need to
                                //We only have to send the sync message, since in the other thread we are receiving every message sent to us
                                match connection_pair.send_message(message.clone()).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        tracing::error!("{}", err);

                                        //Error appeared, after this the tread quits, so there arent an inf amount of threads running
                                        sender.send(None).expect("Failed to signal thread error");
                                        break;
                                    }
                                };
                            }
                        }
                        else
                        {
                            panic!("The message watning to be sent isnt a clientsyncmessage (as required), check what youve modified");
                        }
                    }
                });
            });

            //Try to receive the threads messages
            //Get sent to the channel to be displayed, if the connections errors out, do nothing lol cuz its prolly cuz the sender hadnt done anything
            match self.server_output_receiver.try_recv() {
                Ok(msg) => {
                    //show messages
                    if let Some(message) = msg {
                        //Decrypt the server's reply
                        match decrypt_aes256(&message, &self.client_connection.client_secret) {
                            Ok(decrypted_message) => {
                                let incoming_struct: Result<ServerSync, serde_json::Error> =
                                    serde_json::from_str(&decrypted_message);
                                match incoming_struct {
                                    Ok(msg) => {
                                        //Always make sure to store the latest user_seen list
                                        self.client_ui.incoming_messages.user_seen_list =
                                            msg.user_seen_list;

                                        //If its a sync message then we dont need to back it up
                                        if matches!(
                                            msg.message.message_type,
                                            ServerMessageType::Sync(_)
                                        ) {
                                            return;
                                        }

                                        match &msg.message.message_type {
                                            ServerMessageType::Edit(message) => {
                                                if let Some(new_message) =
                                                    message.new_message.clone()
                                                {
                                                    if let ServerMessageType::Normal(inner) =
                                                        &mut self
                                                            .client_ui
                                                            .incoming_messages
                                                            .message_list
                                                            [message.index as usize]
                                                            .message_type
                                                    {
                                                        inner.message = new_message;
                                                        inner.has_been_edited = true;
                                                    }
                                                }
                                                else {
                                                    self.client_ui.incoming_messages.message_list
                                                        [message.index as usize]
                                                        .message_type = ServerMessageType::Deleted;
                                                }
                                            },
                                            ServerMessageType::Reaction(message) => {
                                                //Search if there has already been a reaction added
                                                match &message.reaction_type {
                                                    crate::app::backend::ReactionType::Add(
                                                        reaction,
                                                    ) => {
                                                        if let Some(index) = self
                                                            .client_ui
                                                            .incoming_messages
                                                            .reaction_list[reaction.message_index]
                                                            .message_reactions
                                                            .iter()
                                                            .position(|item| {
                                                                item.emoji_name
                                                                    == reaction.emoji_name
                                                            })
                                                        {
                                                            //If yes, increment the reaction counter
                                                            self.client_ui
                                                                .incoming_messages
                                                                .reaction_list
                                                                [reaction.message_index]
                                                                .message_reactions[index]
                                                                .authors
                                                                .push(reaction.uuid.clone());
                                                        }
                                                        else {
                                                            //If no, add a new reaction counter
                                                            self.client_ui
                                                                .incoming_messages
                                                                .reaction_list
                                                                [reaction.message_index]
                                                                .message_reactions
                                                                .push(Reaction {
                                                                    emoji_name: reaction
                                                                        .emoji_name
                                                                        .clone(),
                                                                    authors: vec![reaction
                                                                        .uuid
                                                                        .clone()],
                                                                })
                                                        }
                                                    },
                                                    crate::app::backend::ReactionType::Remove(
                                                        reaction,
                                                    ) => {
                                                        //Search for emoji in the emoji list
                                                        //If its not found, it a serious issue, or just internet inconsistency
                                                        if let Some(index) = self
                                                            .client_ui
                                                            .incoming_messages
                                                            .reaction_list[reaction.message_index]
                                                            .message_reactions
                                                            .iter()
                                                            .position(|item| {
                                                                item.emoji_name
                                                                    == reaction.emoji_name
                                                            })
                                                        {
                                                            //Borrow authors list as mutable
                                                            let emoji_authors = &mut self
                                                                .client_ui
                                                                .incoming_messages
                                                                .reaction_list
                                                                [reaction.message_index]
                                                                .message_reactions[index]
                                                                .authors;

                                                            //Remove the user who has sent this message from the authors list
                                                            match emoji_authors.iter().position(
                                                                |uuid| *uuid == reaction.uuid,
                                                            ) {
                                                                Some(idx) => {
                                                                    emoji_authors.remove(idx);
                                                                },
                                                                None => {
                                                                    tracing::error!("Tried to remove a non-author from the authors list.");
                                                                },
                                                            }
                                                            //If the emoji is reacted with 0 times, it means it has been fully deleted from the list
                                                            if emoji_authors.is_empty() {
                                                                self.client_ui
                                                                    .incoming_messages
                                                                    .reaction_list
                                                                    [reaction.message_index]
                                                                    .message_reactions
                                                                    .remove(index);
                                                            }
                                                        }
                                                        else {
                                                            tracing::error!("Emoji was already deleted before requesting removal");
                                                        }
                                                    },
                                                }
                                            },
                                            ServerMessageType::VoipState(state) => {
                                                //Check if the call was alive before the state update
                                                let was_call_alive = self
                                                    .client_ui
                                                    .incoming_messages
                                                    .ongoing_voip_call
                                                    .connected_clients
                                                    .is_none();

                                                //Set state
                                                self.client_ui
                                                    .incoming_messages
                                                    .ongoing_voip_call
                                                    .connected_clients =
                                                    state.connected_clients.clone();
                                            },
                                            _ => {
                                                //Allocate Message vec for the new message
                                                self.client_ui
                                                    .incoming_messages
                                                    .reaction_list
                                                    .push(MessageReaction::default());

                                                //We can append the missing messages sent from the server, to the self.client_ui.incoming_msg.struct_list vector
                                                self.client_ui
                                                    .incoming_messages
                                                    .message_list
                                                    .push(msg.message.clone());

                                                //Callback
                                                // self.client_ui.extension.event_call_extensions(
                                                //     crate::app::lua::EventCall::OnChatReceive,
                                                //     &self.lua,
                                                //     Some(msg.message._struct_into_string()),
                                                // );
                                            },
                                        }
                                    },
                                    //If converting the message to a ServerSync then it was probably a ServerReplyType enum
                                    Err(_err) => {
                                        let incoming_reply: Result<
                                            ServerReplyType,
                                            serde_json::Error,
                                        > = serde_json::from_str(&decrypted_message);

                                        match incoming_reply {
                                            Ok(inner) => {
                                                match inner {
                                                    ServerReplyType::File(file) => {
                                                        // let _ = write_file(file);
                                                    },
                                                    ServerReplyType::Image(image) => {
                                                        //Forget image so itll be able to get displayed
                                                        ctx.forget_image(&format!(
                                                            "bytes://{}",
                                                            image.signature
                                                        ));

                                                        //load image to the said URI
                                                        ctx.include_bytes(
                                                            format!("bytes://{}", image.signature),
                                                            image.bytes,
                                                        );
                                                    },
                                                    ServerReplyType::Audio(audio) => {
                                                        let stream_handle = self
                                                            .client_ui
                                                            .audio_playback
                                                            .stream_handle
                                                            .clone();

                                                        let sender = self.audio_save_tx.clone();

                                                        //ONLY USE THIS PATH WHEN YOU ARE SURE THAT THE FILE SPECIFIED ON THIS PATH EXISTS
                                                        let path_to_audio = PathBuf::from(format!(
                                                            "{}\\Matthias\\Client\\{}\\Audios\\{}",
                                                            env!("APPDATA"),
                                                            self.client_ui
                                                                .send_on_ip_base64_encoded,
                                                            audio.signature
                                                        ));

                                                        let _ = write_audio(
                                                            audio.clone(),
                                                            self.client_ui.send_on_ip.clone(),
                                                        );

                                                        while !path_to_audio.exists() {
                                                            //Block until it exists, we can do this because we are in a different thread then main
                                                        }

                                                        let file_stream_to_be_read =
                                                            fs::read(&path_to_audio)
                                                                .unwrap_or_default();

                                                        let cursor = PlaybackCursor::new(
                                                            file_stream_to_be_read,
                                                        );
                                                        let sink = Some(Arc::new(
                                                            Sink::try_new(&stream_handle).unwrap(),
                                                        ));

                                                        sender
                                                            .send((
                                                                sink,
                                                                cursor,
                                                                //Is this needed
                                                                0,
                                                                path_to_audio,
                                                            ))
                                                            .unwrap();
                                                    },
                                                    ServerReplyType::Client(client_reply) => {
                                                        self.client_ui
                                                            .incoming_messages
                                                            .connected_clients_profile
                                                            .insert(
                                                                client_reply.uuid.clone(),
                                                                client_reply.profile.clone(),
                                                            );

                                                        //Forget old placeholder bytes
                                                        ctx.forget_image(&format!(
                                                            "bytes://{}",
                                                            client_reply.uuid
                                                        ));

                                                        //Pair URI with profile image
                                                        ctx.include_bytes(
                                                            format!(
                                                                "bytes://{}",
                                                                client_reply.uuid
                                                            ),
                                                            client_reply
                                                                .profile
                                                                .small_profile_picture,
                                                        );
                                                    },
                                                }
                                            },
                                            Err(_err) => {
                                                let incoming_reply: Result<
                                                    ServerVoipReply,
                                                    serde_json::Error,
                                                > = serde_json::from_str(&decrypted_message);

                                                match incoming_reply {
                                                    Ok(voip_connection) => {
                                                        match voip_connection {
                                                            ServerVoipReply::Success => {},
                                                            ServerVoipReply::Fail(err) => {
                                                                //Avoid panicking when trying to display a Notification
                                                                //This is very rare but can still happen
                                                                display_error_message(
                                                                    err.reason,
                                                                    self.toasts.clone(),
                                                                );
                                                            },
                                                        }
                                                    },
                                                    Err(_err) => {
                                                        tracing::error!("{}", _err);
                                                    },
                                                }
                                            },
                                        }
                                    },
                                }
                            },
                            Err(err) => {
                                display_error_message(err, self.toasts.clone());

                                //Assuming the connection is faulty we reset state
                                self.reset_client_connection();
                                self.client_connection.reset_state();
                            },
                        }
                    }
                    else {
                        //Signal the remaining thread to be shut down
                        // self.autosync_shutdown_token.cancel();
                        // wtf? investigate

                        //Then the thread got an error, we should reset the state
                        tracing::error!("Client receiver or sync thread panicked");
                    }
                },
                Err(_err) => {
                    // dbg!(_err);
                },
            }
        }
    }
}

/// Receives packets on the given UdpSocket, messages are decrypted with the decrpytion key
/// Automatically appends the decrypted audio bytes to the ```Sink``` if its an uadio packet
/// I might rework this function so that we can see who is talking based on uuid
async fn receive_server_relay(
    //Socket this function is Listening on
    receiver_socket_part: Arc<tokio::net::UdpSocket>,
    //Decryption key
    decryption_key: &[u8],
    //The sink its appending the bytes to
    sink: Arc<Sink>,
    //This serves as the image buffer from the server
    image_buffer: MessageBuffer,

    ctx: &egui::Context,
) -> anyhow::Result<()>
{
    //Create buffer for header, this is the size of the maximum udp packet so no error will appear
    let mut header_buf = vec![0; 65536];

    //Receive header size
    receiver_socket_part
        .peek_from(&mut header_buf)
        .await
        .unwrap();

    //Get message length
    let header_length = u32::from_be_bytes(header_buf[..4].try_into().unwrap());

    //Create body according to message size indicated by the header, make sure to add 4 to the byte length because we peeked the ehader thus we didnt remove the bytes from the buffer
    let mut body_buf = vec![0; (header_length + 4) as usize];

    //Receive the whole message
    receiver_socket_part.recv(&mut body_buf).await.unwrap();

    //Decrypt message
    let mut decrypted_bytes = decrypt_aes256_bytes(
        //Only take the bytes from the 4th byte because thats the header
        &body_buf[4..],
        decryption_key,
    )?;

    let message_flag_bytes: Vec<u8> = decrypted_bytes.drain(decrypted_bytes.len() - 4..).collect();

    match UdpMessageType::from_number(u32::from_be_bytes(message_flag_bytes.try_into().unwrap())) {
        UdpMessageType::Voice => {
            //The generated uuids are always a set amount of bytes, so we can safely extract them, and we know that the the left over bytes are audio
            let uuid = String::from_utf8(
                decrypted_bytes
                    .drain(decrypted_bytes.len() - UUID_STRING_BYTE_LENGTH..)
                    .collect(),
            )?;

            //Make sure to verify that the UUID we are parsing is really a uuid, because if its not we know we have parsed the bytes in an incorrect order
            uuid::Uuid::parse_str(&uuid)
                .map_err(|err| anyhow::Error::msg(format!("Error: {}, in uuid {}", err, uuid)))?;

            //Play received bytes
            sink.append(rodio::Decoder::new(BufReader::new(Cursor::new(
                decrypted_bytes,
            )))?);
        },
        UdpMessageType::ImageHeader => {
            get_image_header(&decrypted_bytes, &image_buffer).unwrap();
        },
        UdpMessageType::Image => {
            // [. . . . . . . . . . . len - 164][len - 164 . . . . . len - 100][len - 100. . . . . len - 64][len - 64 . . . .]
            //      IMAGE                           HASH                            UUID                      IDENTIFICATOR
            let message_bytes = decrypted_bytes.to_vec();

            //Get the identificator of the image part in bytes
            let indetificator_bytes =
                message_bytes[message_bytes.len() - IDENTIFICATOR_BYTE_OFFSET..].to_vec();

            let identificator = String::from_utf8(indetificator_bytes).unwrap();

            //Get the hash of the image part in bytes
            let hash_bytes = message_bytes
                [message_bytes.len() - HASH_BYTE_OFFSET..message_bytes.len() - UUID_BYTE_OFFSET]
                .to_vec();

            let hash = String::from_utf8(hash_bytes).unwrap();

            //Get the image part bytes
            let image = message_bytes[..message_bytes.len() - HASH_BYTE_OFFSET].to_vec();

            let uuid = String::from_utf8(
                message_bytes[message_bytes.len() - UUID_BYTE_OFFSET
                    ..message_bytes.len() - IDENTIFICATOR_BYTE_OFFSET]
                    .to_vec(),
            )
            .unwrap();

            //Make sure to verify that the UUID we are parsing is really a uuid, because if its not we know we have parsed the bytes in an incorrect order
            uuid::Uuid::parse_str(uuid.trim())
                .map_err(|err| anyhow::Error::msg(format!("Error: {}, in uuid {}", err, uuid)))?;

            if let Some(mut image_header) = image_buffer.get_mut(&uuid) {
                if let Some((index, _, contents)) = image_header.get_full_mut(&identificator) {
                    if let Some(byte_pair) = contents.get_mut(&hash) {
                        *byte_pair = Some(image);
                    }
                    else {
                        tracing::error!("Image part hash not found in the image header: {hash}");
                    }

                    //If all the parts of the image header had arrived send the image to all the clients
                    if contents.iter().all(|(_, value)| value.is_some()) {
                        let contents_clone = contents.clone();

                        //Combine the image part bytes
                        let image_bytes: Vec<u8> = contents_clone
                            .iter()
                            .flat_map(|(_, value)| {
                                <std::option::Option<std::vec::Vec<u8>> as Clone>::clone(value)
                                    .unwrap()
                            })
                            .collect();

                        //Define uri
                        let uri = format!("bytes://video_steam:{uuid}");

                        //If the image bytes are empty that means the video stream has shut down
                        if image_bytes == vec![0] {
                            //Forget image on that URI
                            ctx.forget_image(&uri);

                            image_buffer.remove(&uuid);
                        }
                        //Else save the image
                        else {
                            //Forget image on that URI
                            ctx.forget_image(&uri);

                            //Pair URI with bytes
                            ctx.include_bytes(uri, image_bytes);
                        }

                        //Request repaint
                        ctx.request_repaint();

                        //Drain earlier ImageHeaders (and the current one), because a new one has arrived
                        image_header.drain(index..=index);
                    }
                }
                else {
                    tracing::error!("Image header not found: {identificator}");
                }
            }
            else {
                tracing::error!("User not found in the image header list: {uuid}");
            }
        },
    }

    Ok(())
}
