use std::sync::atomic::{AtomicUsize, Ordering};

use v4l::control::MenuItem;
use v4l::prelude::*;
use v4l::context;
use v4l::capability::Flags;
use v4l::video::Capture;
use v4l::{Format, FourCC};
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;

use catppuccin_egui::{FRAPPE, LATTE, MACCHIATO, MOCHA};
use eframe::egui::{self, TextBuffer};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CatppuccinTheme {
    Frappe,
    Latte,
    Macchiato,
    Mocha,
}

struct V4lControl {
    id: u32,
    typ: v4l::control::Type,
    name: String,
    minimum: i64,
    maximum: i64,
    step: u64,
    default: i64,
    flags: v4l::control::Flags,
    items: Option<Vec<(i64, String)>>,
    current: v4l::control::Value,
}

pub struct GuiApp {
    theme: CatppuccinTheme,
    tab: u32,
    device: v4l::Device,
    controls: Vec<V4lControl>,
}

impl GuiApp {
    //cc 
    pub fn new(cc: &eframe::CreationContext<'_>, id: AtomicUsize) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let dev = v4l::Device::new(id.load(Ordering::Relaxed)).expect("v4l device"); 
        let ctrls = Vec::new();

        let mut this = Self {
            theme: CatppuccinTheme::Mocha,
            tab: 0,
            device: dev,
            controls: ctrls,
        };

        this.get_device_ctrls().expect("get device controls");

        this
    }

    fn get_device_ctrls(&mut self) -> Result< i32, i32> {
        
        let caps = self.device.query_caps().expect("device query_caps");
        
        //let vid_cap:bool = (caps.capabilities & Flags::VIDEO_CAPTURE);
        //println!("capabilities VIDEO_CAPTURE: {}\n", vid_cap);
        if !caps.capabilities.intersects(Flags::VIDEO_CAPTURE) {
            return Err(1);
        }
            
        let controls = self.device.query_controls().unwrap();
        
        for ctrl in controls {
            
            let mut value = v4l::control::Value::Integer(0);

            if !ctrl.flags.intersects(v4l::control::Flags::WRITE_ONLY) {
                value = match self.device.control(ctrl.id) {
                    Ok(val) => val.value,
                    Err(er) => {
                        println!("Couldn't get value for ctrl id {}: {}", ctrl.id, er);
                        v4l::control::Value::Integer(0)
                    }
                };
            }

            println!("{}", ctrl);
            match value {
                v4l::control::Value::Integer(val) => {
                    println!("value: Integer({})", val);
                },
                v4l::control::Value::Boolean(val) => {
                    println!("value: Boolean({})", val);
                },
                v4l::control::Value::String(ref val) => {
                    println!("value: String({:?})", val);
                },
                v4l::control::Value::None => {
                    println!("Value: None");
                },
                _ => {
                    println!("value: not supported");
                }
            }

            let ctrl_elem = V4lControl {
                id: ctrl.id,
                typ: ctrl.typ,
                name: ctrl.name.clone(),
                minimum: ctrl.minimum,
                maximum: ctrl.maximum,
                step: ctrl.step,
                default: ctrl.default,
                flags: ctrl.flags,
                items: {
                    if ctrl.items.is_some() {
                        println!("menu items:");
                        let mut menu = Vec::new();
                        let items = ctrl.items.expect("control items vector");
                        for item in items.iter() {
                            let m_value: i64;
                            let (v, m_item) = item;
                            match m_item {
                                MenuItem::Name(name) => {
                                    println!("{}: {}", v, &name);
                                    m_value = *v as i64;
                                    menu.push((m_value, name.clone()));
                                },
                                MenuItem::Value(value) => {
                                    m_value = *value;
                                    println!("{}: {}", v, m_value);
                                    menu.push((m_value, format!("{}", m_value)));
                                }
                            }
                        }
                        Some(menu)
                    } else {
                        None
                    }
                },
                current: value,
            };
    
            self.controls.push(ctrl_elem);

            println!("-----------------------------");

        }

        Ok(0)
    }

    fn update_controls(&mut self) -> Result<i32, i32> {
        let q_ctrls = self.device.query_controls().expect("query controls");

        for (pos, ctrl) in self.controls.iter_mut().enumerate() {
            let mut value = v4l::control::Value::Integer(0);
            ctrl.flags = q_ctrls[pos].flags;

            if !ctrl.flags.intersects(v4l::control::Flags::WRITE_ONLY) {
                value = match self.device.control(ctrl.id) {
                    Ok(val) => val.value,
                    Err(er) => {
                        println!("Couldn't get value for ctrl id {}: {}", ctrl.id, er);
                        v4l::control::Value::Integer(0)
                    }
                };
            }

            ctrl.current = value;

         //   println!("{}", q_ctrls[pos]);
         //   match ctrl.current {
         //       v4l::control::Value::Integer(v) => {
         //           println!("value: Integer({})", v);
         //       },
         //       v4l::control::Value::Boolean(v) => {
         //           println!("value: Boolean({})", v);
         //       },
         //       v4l::control::Value::String(ref v) => {
         //           println!("value: String({:?})", v);
         //       },
         //       v4l::control::Value::None => {
         //           println!("Value: None");
         //       },
         //       _ => {
         //           println!("value: not supported");
         //       }
         //   }
         //   println!("-----------------------------");

        }

        Ok(0)
    }

    fn gui_controls(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
    
        egui::ScrollArea::vertical()
            .max_height(
                ui.available_height() - ui.text_style_height(&egui::TextStyle::Body) * 2.0,
            )
            .show(ui, |ui| {

                let mut changed = false;

                ui.spacing_mut().slider_width = 300.;

                ui.set_width(ui.available_width());

                for ctrl in self.controls.iter_mut() {
                    match ctrl.typ {
                        v4l::control::Type::CtrlClass => {
                            ui.separator();
                            ui.heading(ctrl.name.clone());
                        },
                        
                        v4l::control::Type::String => {
                        },

                        v4l::control::Type::Boolean => {
                            let mut c_value: bool = match ctrl.current {
                                v4l::control::Value::Boolean(v) => v,
                                _ => {
                                    println!("Bad bool control value: setting to false");
                                    false
                                },
                            };

                            let disabled = ctrl.flags.intersects(
                                v4l::control::Flags::INACTIVE.union(v4l::control::Flags::DISABLED));

                            let response = ui.add_enabled(
                                !disabled,
                                egui::Checkbox::new(&mut c_value, ctrl.name.clone()));
                            if response.clicked() {
                                //println!("control id {} changed to {}", ctrl.id, val);
                                ctrl.current = v4l::control::Value::Boolean(c_value);
                                let control = v4l::Control {
                                    id: ctrl.id, 
                                    value: v4l::control::Value::Boolean(c_value),
                                };
                                let _ = self.device.set_control(control);
                                changed = true;
                            };
                        },

                        v4l::control::Type::U8 |
                        v4l::control::Type::U16 |
                        v4l::control::Type::U32 |
                        v4l::control::Type::Integer |
                        v4l::control::Type::Integer64 => {
                            let mut c_value = match ctrl.current {
                                v4l::control::Value::Integer(val) => val,
                                _ => {
                                    println!("Bad value: expected Integer - set to 0");
                                    0
                                },
                            };

                            let disabled = ctrl.flags.intersects(
                                v4l::control::Flags::INACTIVE.union(v4l::control::Flags::DISABLED));
                            
                            let response = ui.add_enabled(
                                !disabled,
                                egui::Slider::new(&mut c_value, ctrl.minimum..=ctrl.maximum)
                                    .text(ctrl.name.clone()));
                            if response.changed() {
                                //println!("control id {} changed to {}", ctrl.id, val);
                                ctrl.current = v4l::control::Value::Integer(c_value);
                                let control = v4l::Control {
                                    id: ctrl.id, 
                                    value: v4l::control::Value::Integer(c_value),
                                };
                                let _ = self.device.set_control(control);
                                changed = true;
                            };
                        },

                        v4l::control::Type::Button => {
                        },

                        v4l::control::Type::Menu => {
                            let mut val = 0;
                            if let v4l::control::Value::Integer(value) = ctrl.current {
                                val = value;
                            }

                            let mut select_ind = 0;

                            if let Some(items) = &ctrl.items { 
                                for (pos, item) in items.iter().enumerate() {
                                    if val == item.0 {
                                        select_ind = pos;
                                        break;
                                    } 
                                }

                                let name_selected = items[select_ind].1.clone();

                                egui::ComboBox::from_label(ctrl.name.clone())
                                    .selected_text(format!("{:?}", name_selected))
                                    .show_ui(ui, |ui| {
                                        for (val, name) in items.iter() {
                                            let mut selected: i64 = 0;
                                            let response = ui.selectable_value(&mut selected, *val, format!("{:?}", name));
                                            if response.clicked() {
                                                ctrl.current = v4l::control::Value::Integer(selected);
                                                let control = v4l::Control {
                                                    id: ctrl.id, 
                                                    value: v4l::control::Value::Integer(selected),
                                                };
                                                let _ = self.device.set_control(control);
                                                changed = true;
                                            }
                                        }
                                    });
                            };
                              // .show_index(
                              //     ui,
                              //     &mut usize::try_from(ctrl.current).expect("usize conversion from u64"),
                              //     ctrl.items.expect("menu items").len(),
                              //     |i| ctrl.items.expect("menu items") [i]
                              // );
                        },
                        _ => (),
                    }
                }

                if changed {
                    self.update_controls().expect("update controls");
                }
            })
    }

    fn gui_settings(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
    
        egui::ScrollArea::vertical()
            .max_height(
                ui.available_height() - ui.text_style_height(&egui::TextStyle::Body) * 2.0,
            )
            .show(ui, |ui| {

                ui.spacing_mut().slider_width = 300.;

                ui.set_width(ui.available_width());

                ui.separator();
            })
    }

}

impl eframe::App for GuiApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        if self.controls.is_empty() {
            self.get_device_ctrls().expect("update controls");
        }

        catppuccin_egui::set_theme(
            ctx,
            match self.theme {
                CatppuccinTheme::Frappe => FRAPPE,
                CatppuccinTheme::Latte => LATTE,
                CatppuccinTheme::Macchiato => MACCHIATO,
                CatppuccinTheme::Mocha => MOCHA,
            },
        );

        ctx.set_pixels_per_point(1.25);
 
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                
                //ui.style_mut().text_styles.insert(
                //    egui::TextStyle::Button,
                //    egui::FontId::new(24.0, eframe::epaint::FontFamily::Proportional),
                //);

                ui.columns(2, |columns| {
                    columns[0].heading("");
                    columns[1].with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        egui::ComboBox::from_label("Theme:")
                            .selected_text(format!("{:?}", self.theme))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.theme, CatppuccinTheme::Latte, "Latte");
                                ui.selectable_value(&mut self.theme, CatppuccinTheme::Frappe, "Frappe");
                                ui.selectable_value(
                                    &mut self.theme,
                                    CatppuccinTheme::Macchiato,
                                    "Macchiato",
                                );
                                ui.selectable_value(&mut self.theme, CatppuccinTheme::Mocha, "Mocha");
                            });
                    });
                });

                ui.columns(2, |columns| {
                    columns[0].with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Button,
                            egui::FontId::new(24.0, eframe::epaint::FontFamily::Proportional),
                        );
      
                        let tab0 = egui::Button::new("Controls")
                            .selected(self.tab == 0);
                        
                        let tab1 = egui::Button::new("Settings")
                            .selected(self.tab == 1);
        
                        if ui.add_sized([120., 40.], tab0).clicked() {
                            self.tab = 0;
                        }

                        if ui.add_sized([120., 40.], tab1).clicked() {
                            self.tab = 1;
                        }
                    });
                    columns[1].heading("");
                });
                
                if self.tab == 1 {
                    self.gui_settings(ui);
                } else {
                    self.gui_controls(ui);
                }                  
        });
    }
}
