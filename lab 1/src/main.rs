use serde::Deserialize;
use windows::Win32::System::Com::{
    CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use wmi::{COMLibrary, WMIConnection};

#[derive(Default)]
struct LabApp {
    com_lib: Option<COMLibrary>,
    wmi_con: Option<WMIConnection>,
    env_vars: Vec<String>,
    sid_counts: Vec<String>,
    bus_info: Vec<String>,
    active_data: ActiveData, // Новое поле для отслеживания активных данных
}

#[derive(Default, PartialEq)]
enum ActiveData {
    #[default]
    None,
    EnvVars,
    SidCounts,
    BusInfo,
}
#[derive(Debug, Deserialize)]
#[serde(rename = "Win32_Environment")]
struct Win32Environment {
    Name: String,
    VariableValue: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Win32_Account")]
struct Win32Account {
    SIDType: u8,
    Caption: String,
}

// Исправленная структура для шин (класс может отличаться в вашей системе)
#[derive(Debug, Deserialize)]
#[serde(rename = "Win32_PnPEntity")] // Пример альтернативного класса
struct Win32Bus {
    DeviceID: String,
    Status: String, // Пример другого поля
}

impl LabApp {
    fn init_wmi(&mut self) -> Result<(), wmi::WMIError> {
        unsafe {
            let coinit_flags = COINIT_APARTMENTTHREADED.0 | COINIT_DISABLE_OLE1DDE.0;
            let hres = CoInitializeEx(None, std::mem::transmute(coinit_flags));

            if hres.is_err() {
                return Err(wmi::WMIError::HResultError {
                    hres: hres.0 as i32,
                });
            }
            self.com_lib = Some(COMLibrary::assume_initialized());
        }
        self.wmi_con = Some(WMIConnection::with_namespace_path(
            "root\\cimv2",
            self.com_lib.as_ref().unwrap().clone(),
        )?);

        Ok(())
    }
}

impl eframe::App for LabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Лабораторная работа — WMI");

            if self.wmi_con.is_none() {
                if let Err(e) = self.init_wmi() {
                    ui.label(format!("Ошибка инициализации WMI: {e}"));
                    return;
                }
            }

            // Группа кнопок
            ui.horizontal(|ui| {
                if ui.button("Переменные окружения").clicked() {
                    self.active_data = ActiveData::EnvVars;
                    self.env_vars.clear();
                    if let Some(wmi_con) = &self.wmi_con {
                        match wmi_con.query::<Win32Environment>() {
                            Ok(envs) => {
                                self.env_vars = envs
                                    .iter()
                                    .map(|env| format!("{}: {}", env.Name, env.VariableValue))
                                    .collect();
                            }
                            Err(e) => {
                                self.env_vars.push(format!("Ошибка: {e}"));
                            }
                        }
                    }
                }

                if ui.button("Статистика SID").clicked() {
                    self.active_data = ActiveData::SidCounts;
                    self.sid_counts.clear();
                    if let Some(wmi_con) = &self.wmi_con {
                        match wmi_con.query::<Win32Account>() {
                            Ok(accounts) => {
                                let mut counts = std::collections::HashMap::new();
                                accounts.iter().for_each(|acc| {
                                    *counts.entry(acc.SIDType).or_insert(0) += 1;
                                });
                                self.sid_counts = counts
                                    .iter()
                                    .map(|(k, v)| format!("Тип {}: {}", k, v))
                                    .collect();
                            }
                            Err(e) => {
                                self.sid_counts.push(format!("Ошибка: {e}"));
                            }
                        }
                    }
                }

                if ui.button("Информация о шинах").clicked() {
                    self.active_data = ActiveData::BusInfo;
                    self.bus_info.clear();
                    if let Some(wmi_con) = &self.wmi_con {
                        match wmi_con.query::<Win32Bus>() {
                            Ok(buses) => {
                                self.bus_info = buses
                                    .iter()
                                    .map(|bus| {
                                        format!("ID: {}, Статус: {}", bus.DeviceID, bus.Status)
                                    })
                                    .collect();
                            }
                            Err(e) => {
                                self.bus_info.push(format!("Ошибка: {e}"));
                            }
                        }
                    }
                }
            });

            // Отображение результатов
            ui.separator();
            ui.label("Результаты:");

            egui::ScrollArea::vertical()
                .id_salt("results_scroll")
                .show(ui, |ui| match self.active_data {
                    ActiveData::EnvVars => {
                        for env in &self.env_vars {
                            ui.label(env);
                        }
                    }
                    ActiveData::SidCounts => {
                        for sid in &self.sid_counts {
                            ui.label(sid);
                        }
                    }
                    ActiveData::BusInfo => {
                        for bus in &self.bus_info {
                            ui.label(bus);
                        }
                    }
                    ActiveData::None => {
                        ui.label("Выберите категорию для отображения данных");
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "WMI Lab",
        options,
        Box::new(|_cc| Ok(Box::new(LabApp::default()))),
    )
}
