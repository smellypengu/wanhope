pub mod ui;

use std::collections::{BTreeSet, HashMap};

use crate::{
    app::AppError,
    graphics::{
        vulkan::{EGuiIntegration, Renderer},
        RenderError, Window,
    },
    network::Network,
};

pub struct Props<'a> {
    network: &'a Network,
}

pub trait Ui {
    fn name(&self) -> &'static str;

    fn show(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        props: &Props,
    ) -> Option<egui::InnerResponse<Option<anyhow::Result<(), AppError>>>>;

    fn as_any(&mut self) -> &mut dyn std::any::Any;
}

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui, props: &Props) -> anyhow::Result<(), AppError>;
}

pub struct EGui {
    egui_integration: EGuiIntegration,

    pub uis: HashMap<String, Box<dyn Ui>>,

    open: BTreeSet<String>,
}

impl EGui {
    pub fn new(window: &Window, renderer: &Renderer) -> anyhow::Result<Self, AppError> {
        let egui_integration = EGuiIntegration::new(
            window,
            renderer.device.clone(),
            &renderer.swapchain,
            renderer.swapchain.swapchain_image_format,
        )?;

        let uis_vec: Vec<Box<dyn Ui>> = vec![Box::new(ui::Chat::default())];

        let mut uis = HashMap::new();

        for ui in uis_vec {
            uis.insert(ui.name().to_owned(), ui);
        }

        Ok(Self {
            egui_integration,

            uis,

            open: BTreeSet::new(),
        })
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.egui_integration.on_event(event)
    }

    pub unsafe fn render(
        &mut self,
        window: &Window,
        renderer: &Renderer,
        command_buffer: ash::vk::CommandBuffer,
        network: &mut Network,
        world: &Option<common::world::World>,
    ) -> anyhow::Result<(), AppError> {
        self.egui_integration.begin_frame(window);

        egui::TopBottomPanel::top("top_panel").show(
            &self.egui_integration.egui_ctx.clone(),
            |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| if ui.button("Test").clicked() {});
                });
            },
        );

        egui::SidePanel::left("side_panel")
            .show(
                &self.egui_integration.egui_ctx.clone(),
                |ui| -> anyhow::Result<(), AppError> {
                    ui.heading("Wanhope");
                    ui.separator();

                    if !network.connected {
                        ui.horizontal(|ui| {
                            ui.label("IP: ");
                            ui.text_edit_singleline(&mut network.ip);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Username: ");
                            ui.text_edit_singleline(&mut network.username);
                        });

                        if ui.button("Connect").clicked() {
                            match network.join() {
                                Ok(_) => {}
                                Err(err) => {
                                    ui.label(format!("Failed to join server: {}", err));
                                }
                            }
                        };
                    } else {
                        ui.label(format!(
                            "Connected to {} as {}",
                            network.server_ip().unwrap(),
                            network.client_id.unwrap(),
                        ));

                        ui.label("Players:");
                        if let Some(world) = world {
                            for player in &world.players {
                                if let Some(player) = player {
                                    ui.label(format!("{}", player.username));
                                }
                            }
                        }

                        ui.separator();

                        if ui.button("Leave").clicked() {
                            match network.leave() {
                                Ok(_) => {
                                    self.set_open("Chat", false);
                                }
                                Err(_) => {
                                    // TODO: handle better?
                                    log::warn!("Failed to leave server");
                                }
                            }
                        }

                        if ui.button("Chat").clicked() {
                            self.toggle_open("Chat");
                        };
                    }

                    ui.separator();

                    Ok(())
                },
            )
            .inner?;

        let props = Props { network: &network };

        for ui in &mut self.uis {
            let mut is_open = self.open.contains(ui.1.name());

            if let Some(r) = ui.1.show(
                &self.egui_integration.egui_ctx.clone(),
                &mut is_open,
                &props,
            ) {
                match r.inner {
                    Some(inner) => inner?,
                    None => {}
                }
            }
        }

        self.egui_integration.end_frame(window);

        self.egui_integration
            .paint(command_buffer, renderer.image_index())?;

        Ok(())
    }

    pub unsafe fn resize(
        &mut self,
        window: &Window,
        renderer: &Renderer,
    ) -> anyhow::Result<(), RenderError> {
        self.egui_integration.update_swapchain(
            &window,
            &renderer.swapchain,
            renderer.swapchain.swapchain_image_format,
        )
    }

    fn set_open(&mut self, key: &'static str, is_open: bool) {
        if is_open && !self.open.contains(key) {
            self.open.insert(key.to_owned());
        } else {
            self.open.remove(key);
        }
    }

    pub fn toggle_open(&mut self, key: &'static str) {
        if !self.open.contains(key) {
            self.open.insert(key.to_owned());
        } else {
            self.open.remove(key);
        }
    }

    pub fn get_mut<T: 'static>(&mut self, key: &'static str) -> Option<&mut T>
    where
        T: Ui,
    {
        self.uis.get_mut(key)?.as_any().downcast_mut::<T>()
    }
}
