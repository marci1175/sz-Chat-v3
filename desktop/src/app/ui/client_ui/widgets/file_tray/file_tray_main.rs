use egui::{vec2, Align, Color32, ImageButton, Layout, RichText};

//use crate::app::account_manager::write_file;
use crate::app::backend::{Application, MessagingMode, ServerMessageType};

impl Application
{
    pub fn file_tray(&mut self, ctx: &egui::Context)
    {
        egui::TopBottomPanel::bottom("file_tray").show_animated(ctx, (!self.client_ui.files_to_send.is_empty() || matches!(self.client_ui.messaging_mode, MessagingMode::Reply(_)) || matches!(self.client_ui.messaging_mode, MessagingMode::Edit(_))) && self.client_ui.usr_msg_expanded, |ui|{
            ui.allocate_space(vec2(ui.available_width(), 10.));
                egui::ScrollArea::horizontal()
                        .id_source("file_to_send")
                        .stick_to_right(true)
                        .show(ui, |ui|{
                            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                                for (index, item) in self.client_ui.files_to_send.clone().iter().enumerate() {
                                    ui.group(|ui| {
                                        ui.allocate_ui(vec2(200., 100.), |ui| {
                                            ui.with_layout(Layout::left_to_right(Align::Center), |ui|{
                                                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                                                    //file icon
                                                    ui.allocate_ui(vec2(75., 75.), |ui|{
                                                        match item.extension().unwrap().to_string_lossy().to_ascii_lowercase().as_str() {
                                                            //file extenisons
                                                            "exe" | "msi" | "cmd" | "com" | "inf" | "bat" | "ipa" | "osx" | "pif" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/exe_icon.png")));
                                                            }
                                                            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "z" | "tgz" | "tbz2" | "txz" | "sit" | "tar.gz" | "tar.bz2" | "tar.xz" | "zipp" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/zip_icon.png")));
                                                            }
                                                            "jpeg" | "jpg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "svg" | "ico" | "raw" | "heif" | "pdf" | "eps" | "ai" | "psd" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/picture_icon.png")));
                                                            }
                                                            "wav" | "mp3" | "ogg" | "flac" | "aac" | "midi" | "wma" | "aiff" | "ape" | "alac" | "amr" | "caf" | "au" | "ra" | "m4a" | "ac3" | "dts" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/sound_icon.png")));
                                                            }
                                                            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "3gp" | "mpeg" | "mpg" | "rm" | "swf" | "vob" | "ts" | "m2ts" | "mts" | "divx" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/video_icon.png")));
                                                            }
                                                            // :)
                                                            "rs" => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/rust_lang_icon.png")));
                                                            }
                                                            _ => {
                                                                ui.add(egui::widgets::Image::new(egui::include_image!("../../../../../../../assets/icons/file_types/general_icon.png")));
                                                            }
                                                        }
                                                    });
                                                    //selected file widget part
                                                    ui.label(
                                                        RichText::from(
                                                            item.file_name()
                                                                .unwrap_or_default()
                                                                .to_string_lossy(),
                                                        )
                                                        .size(self.font_size),
                                                    );
                                                });
                                                ui.separator();
                                                //bin icon
                                                ui.allocate_ui(vec2(30., 30.), |ui|{
                                                    if ui.add(
                                                        ImageButton::new(
                                                            egui::include_image!("../../../../../../../assets/icons/delete.png")
                                                        )
                                                    ).clicked() {
                                                        self.client_ui.files_to_send.remove(index);
                                                    };
                                                });
                                            });
                                        });
                                    });
                                }
                            });
                });
                match self.client_ui.messaging_mode {
                    MessagingMode::Edit(edit_index) => {
                        if !self.client_ui.files_to_send.is_empty() {
                            ui.separator();
                        }
                        ui.horizontal(|ui| {
                            ui.group(|ui|{
                                //Editing message ui part
                                ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                                    //place them in one line
                                    //Selected message
                                    let selected_message = &self.client_ui.incoming_messages.message_list[edit_index];
                                    ui.horizontal(|ui| {
                                        //Editing: {msg}
                                        ui.label(RichText::from(match &selected_message.message_type {
                                            //We only have to display this enum variant cuz thats the only one which can be edited
                                            ServerMessageType::Normal(msg) => format!("Editing: {}", {
                                                let mut msg = msg.message.clone();

                                                //Truncate message if its too long
                                                if msg.len() > 20 {
                                                    msg.truncate(20);
                                                    msg.push_str(". . .");
                                                }

                                                msg
                                            }),
                                            _ => { unimplemented!() }
                                        }).size(self.font_size).strong());
                                    });
                                });
                            });
                            if ui.add(egui::ImageButton::new(egui::include_image!("../../../../../../../assets/icons/cross.png"))).clicked() {
                                //Reset messaging mode
                                self.client_ui.messaging_mode = MessagingMode::Normal;
                                //Clear messaging buffer
                                self.client_ui.message_buffer.clear();
                                self.client_ui.text_edit_cursor_index = 0;
                            }
                        });
                    }
                    MessagingMode::Reply(replying_to) => {
                        if !self.client_ui.files_to_send.is_empty() {
                            ui.separator();
                        }
                        ui.horizontal(|ui| {
                            ui.group(|ui|{
                                //Replying to ui part
                                ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                                    //place them in one line
                                    //Selected message
                                    let selected_message = &self.client_ui.incoming_messages.message_list[replying_to];
                                    ui.horizontal(|ui| {
                                        //Replying to "{author}:"
                                        ui.label(RichText::from(format!("{}:", selected_message.author)).size(self.font_size).weak().color(Color32::LIGHT_GRAY));
                                        ui.label(RichText::from(match &selected_message.message_type {
                                            ServerMessageType::Deleted => "Deleted message".to_string(),
                                            ServerMessageType::Audio(audio) => format!("Sound {}", audio.file_name),
                                            ServerMessageType::Image(_img) => "Image".to_string(),
                                            ServerMessageType::Upload(upload) => format!("Upload {}", upload.file_name),
                                            ServerMessageType::Normal(msg) => msg.message.clone(),
                                            ServerMessageType::Server(server) => match server {
                                                crate::app::backend::ServerMessage::Connect(profile) => {
                                                    format!("{} has connected", profile.username)
                                                },
                                                crate::app::backend::ServerMessage::Disconnect(profile) => {
                                                    format!("{} has disconnected", profile.username)
                                                },
                                                crate::app::backend::ServerMessage::Ban(profile) => {
                                                    format!("{} has been banned", profile.username)
                                                },
                                            },
                                            ServerMessageType::VoipEvent(_) => unreachable!(),
                                            ServerMessageType::Edit(_) => unreachable!(),
                                            ServerMessageType::Reaction(_) => unreachable!(),
                                            ServerMessageType::Sync(_) => unreachable!(),
                                                                ServerMessageType::VoipState(_) => unreachable!(),
                                                            }).size(self.font_size).strong());
                                    });
                                });
                            });
                            if ui.add(egui::ImageButton::new(egui::include_image!("../../../../../../../assets/icons/cross.png"))).clicked() {
                                self.client_ui.messaging_mode = MessagingMode::Normal;
                            }
                        });
                    }
                    MessagingMode::Normal => {},
                }
                ui.allocate_space(vec2(ui.available_width(), 10.));
            });
    }
}
