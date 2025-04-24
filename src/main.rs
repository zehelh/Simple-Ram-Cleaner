use anyhow::Result;
use chrono::Local;
use egui::{RichText, Align, Align2, Layout, TextStyle, Vec2, Color32, Rounding, Sense};
use poll_promise::Promise;
use image::{ImageBuffer, Rgba, load_from_memory};
use eframe::IconData;
use windows::{
    Win32::{
        Foundation::{BOOL, CloseHandle, HMODULE, INVALID_HANDLE_VALUE, MAX_PATH},
        System::{
            ProcessStatus::{EnumProcesses, GetProcessMemoryInfo, GetModuleBaseNameW, EmptyWorkingSet},
            Threading::{GetCurrentProcess, OpenProcess, PROCESS_ALL_ACCESS},
        },
    },
};

// Logo intégré en tant que ressource
const LOGO_BYTES: &[u8] = include_bytes!("../logo.png");

#[repr(C)]
struct PROCESS_MEMORY_COUNTERS {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    page_file_usage: usize,
    peak_page_file_usage: usize,
}

// Structure pour stocker les informations d'un processus nettoyé
#[derive(Clone)]
struct CleanedProcess {
    name: String,
    memory_freed: usize,
}

// Structure pour stocker les résultats du nettoyage
#[derive(Clone)]
struct CleaningResults {
    processes: Vec<CleanedProcess>,
    cleaned_count: usize,
    total_memory_before: usize,
    total_memory_after: usize,
    global_clean_success: bool,
    start_time: chrono::DateTime<Local>,
    end_time: Option<chrono::DateTime<Local>>,
    is_completed: bool,
    has_error: bool,
    error_message: String,
}

impl CleaningResults {
    fn new() -> Self {
        CleaningResults {
            processes: Vec::new(),
            cleaned_count: 0,
            total_memory_before: 0,
            total_memory_after: 0,
            global_clean_success: false,
            start_time: Local::now(),
            end_time: None,
            is_completed: false,
            has_error: false,
            error_message: String::new(),
        }
    }

    fn total_freed(&self) -> usize {
        if self.total_memory_before > self.total_memory_after {
            self.total_memory_before - self.total_memory_after
        } else {
            0
        }
    }
}

// Structure principale pour l'application
struct CleanRamApp {
    cleaning_promise: Option<Promise<Result<CleaningResults, String>>>,
    last_results: Option<CleaningResults>,
    show_admin_error: bool,
    cleaning_progress: f32,
    system_memory_info: (usize, usize),
    logo_texture: Option<egui::TextureHandle>,
}

impl CleanRamApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            cleaning_promise: None,
            last_results: None,
            show_admin_error: false,
            cleaning_progress: 0.0,
            system_memory_info: (0, 0),
            logo_texture: None,
        }
    }

    // Chargement du logo
    fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_none() {
            // Charger le logo intégré
            if let Ok(image) = load_from_memory(LOGO_BYTES) {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.into_raw();
                self.logo_texture = Some(ctx.load_texture(
                    "logo",
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    egui::TextureOptions::default(),
                ));
            }
        }
    }

    fn start_cleaning(&mut self) {
        if self.cleaning_promise.is_some() {
            return; // Ne pas démarrer un nouveau nettoyage si un est en cours
        }

        self.cleaning_progress = 0.0; // Réinitialiser la progression
        self.cleaning_promise = Some(Promise::spawn_thread("cleaning", || {
            match clean_memory() {
                Ok(results) => Ok(results),
                Err(e) => {
                    let mut results = CleaningResults::new();
                    results.has_error = true;
                    results.error_message = e.to_string();
                    results.is_completed = true;
                    results.end_time = Some(Local::now());
                    Ok(results)
                }
            }
        }));
    }
}

impl eframe::App for CleanRamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Chargement du logo si nécessaire
        self.load_logo(ctx);
        
        // Mettre à jour les informations sur la mémoire système
        self.system_memory_info = get_system_memory_info();
        
        // Vérifier si le nettoyage est terminé
        if let Some(promise) = &self.cleaning_promise {
            // Vérifier si la promesse est prête
            if let Some(result) = promise.ready() {
                // Stocker les résultats et réinitialiser la promesse
                if let Ok(results) = result {
                    self.last_results = Some(results.clone());
                    // Définir la progression à 100% pour indiquer que le nettoyage est terminé
                    self.cleaning_progress = 1.0;
                }
                // Réinitialiser la promesse pour permettre un nouveau nettoyage
                self.cleaning_promise = None;
            } else {
                // Si le nettoyage est en cours mais pas encore terminé, incrémenter la progression
                if self.cleaning_progress < 0.95 {
                    self.cleaning_progress += 0.01; // Incrémenter progressivement jusqu'à 95%
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                
                // Afficher le logo et le titre côte à côte
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    
                    // Afficher le logo s'il est chargé
                    if let Some(texture) = &self.logo_texture {
                        let logo_size = Vec2::new(32.0, 32.0);
                        ui.image(texture, logo_size);
                        ui.add_space(10.0);
                    }
                    
                    ui.heading("Simple RAM Cleaner");
                    ui.add_space(10.0);
                });
                
                ui.add_space(10.0);
                
                // Afficher les informations système
                let (total, avail) = self.system_memory_info;
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Mémoire système:");
                    ui.label(format!("{} total, {} disponible", format_size(total), format_size(avail)));
                });
                ui.add_space(10.0);

                // Bouton de nettoyage amélioré
                if self.cleaning_promise.is_none() {
                    // Zone pour le bouton personnalisé
                    let button_text = "Nettoyer la mémoire cache";
                    let button_size = Vec2::new(250.0, 40.0);
                    let (rect, response) = ui.allocate_exact_size(button_size, Sense::click());
                    
                    let mut normal_color = Color32::from_rgb(30, 144, 255);  // Bleu normal
                    let hover_color = Color32::from_rgb(20, 100, 200);      // Bleu plus foncé au survol
                    
                    // Changer la couleur si survolé
                    if response.hovered() {
                        normal_color = hover_color;
                    }
                    
                    // Dessiner le fond du bouton
                    ui.painter().rect_filled(
                        rect,
                        Rounding::same(5.0),
                        normal_color,
                    );
                    
                    // Dessiner le texte blanc centré
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        button_text,
                        TextStyle::Button.resolve(ui.style()),
                        Color32::WHITE,
                    );
                    
                    if response.clicked() {
                        if !is_elevated::is_elevated() {
                            self.show_admin_error = true;
                        } else {
                            self.start_cleaning();
                        }
                    }
                } else {
                    // Afficher une barre de progression et l'état du nettoyage
                    ui.add_space(5.0);
                    let progress_bar = egui::widgets::ProgressBar::new(self.cleaning_progress)
                        .animate(true)
                        .show_percentage()
                        .desired_width(250.0);
                    ui.add(progress_bar);
                    
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.vertical_centered_justified(|ui| {
                            ui.label(
                                RichText::new("Nettoyage en cours...")
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(30, 144, 255))
                            );
                        });
                    });
                }
                
                // Affichage des résultats du nettoyage
                if let Some(results) = &self.last_results {
                    ui.add_space(15.0);
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.heading("Résultats du nettoyage");
                        ui.horizontal(|ui| {
                            ui.label("Mémoire libérée:");
                            ui.label(
                                RichText::new(format_size(results.total_freed()))
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 180, 0))
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Processus nettoyés:");
                            ui.label(RichText::new(format!("{}", results.processes.len())).strong());
                        });
                        ui.horizontal(|ui| {
                            let elapsed = if let Some(end_time) = results.end_time {
                                (end_time - results.start_time).num_milliseconds() as f32 / 1000.0
                            } else {
                                0.0
                            };
                            ui.label("Temps de nettoyage:");
                            ui.label(RichText::new(format!("{:.2}s", elapsed)).strong());
                        });
                        
                        // Montrer plus de détails sur les processus nettoyés
                        ui.collapsing("Détails des processus", |ui| {
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                let mut cleaned_processes = results.processes.clone();
                                cleaned_processes.sort_by(|a, b| b.memory_freed.cmp(&a.memory_freed));
                                
                                for process in cleaned_processes {
                                    if process.memory_freed > 0 {
                                        ui.horizontal(|ui| {
                                            ui.label(&process.name);
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(format_size(process.memory_freed));
                                            });
                                        });
                                    }
                                }
                            });
                        });
                    });
                }
                
                // Affichage du message d'erreur administrateur
                if self.show_admin_error {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("⚠️ Cette application nécessite des droits administrateur pour fonctionner correctement.")
                            .color(egui::Color32::from_rgb(255, 100, 100))
                    );
                    ui.label("Veuillez la redémarrer en tant qu'administrateur.");
                }

                // Pied de page avec informations de version
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .text_style(TextStyle::Small)
                            .color(egui::Color32::from_rgb(128, 128, 128))
                    );
                });
            });
        });
        
        // Demander une mise à jour continue pendant le nettoyage
        if self.cleaning_promise.is_some() {
            ctx.request_repaint();
        }
    }
}

// Fonction principale pour nettoyer la mémoire
fn clean_memory() -> Result<CleaningResults, String> {
    let mut results = CleaningResults::new();

    // Obtenir les processus
    let mut processes = Vec::with_capacity(1024);
    processes.resize(1024, 0);
    let mut bytes_needed = 0;

    unsafe {
        if EnumProcesses(
            processes.as_mut_ptr(),
            (processes.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_needed,
        ) == BOOL(0)
        {
            return Err("Échec de l'énumération des processus".to_string());
        }
    }

    let process_count = bytes_needed as usize / std::mem::size_of::<u32>();
    let processes = &processes[0..process_count];

    // Libération globale de la mémoire système
    let current_process = unsafe { GetCurrentProcess() };
    
    // Utiliser EmptyWorkingSet pour nettoyer le processus actuel
    results.global_clean_success = unsafe { EmptyWorkingSet(current_process) }.as_bool();

    // Pour chaque processus
    for &pid in processes {
        if pid == 0 {
            continue;
        }

        // Ouvrir un handle vers le processus avec accès complet
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid) };

        if let Ok(handle) = handle {
            if handle != INVALID_HANDLE_VALUE {
                // Essayer d'obtenir le nom du processus
                let mut name_buffer = [0u16; MAX_PATH as usize];
                let name_len = unsafe { 
                    GetModuleBaseNameW(
                        handle, 
                        HMODULE(0), 
                        &mut name_buffer
                    )
                };

                let process_name = if name_len > 0 {
                    String::from_utf16_lossy(&name_buffer[..name_len as usize])
                } else {
                    format!("PID: {}", pid)
                };

                // Obtenir la mémoire avant le nettoyage
                let mut mem_counters = PROCESS_MEMORY_COUNTERS {
                    cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                    page_fault_count: 0,
                    peak_working_set_size: 0,
                    working_set_size: 0,
                    quota_peak_paged_pool_usage: 0,
                    quota_paged_pool_usage: 0,
                    quota_peak_non_paged_pool_usage: 0,
                    quota_non_paged_pool_usage: 0,
                    page_file_usage: 0,
                    peak_page_file_usage: 0,
                };

                let before_memory = unsafe { 
                    if GetProcessMemoryInfo(
                        handle, 
                        &mut mem_counters as *mut PROCESS_MEMORY_COUNTERS as *mut _, 
                        std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32
                    ) != BOOL(0) {
                        mem_counters.working_set_size
                    } else {
                        0
                    }
                };

                results.total_memory_before += before_memory;

                // Tenter le nettoyage de la mémoire du processus avec EmptyWorkingSet
                let success = unsafe { EmptyWorkingSet(handle) };

                if success != BOOL(0) {
                    // Mesurer à nouveau la mémoire après le nettoyage
                    let after_memory = unsafe { 
                        if GetProcessMemoryInfo(
                            handle, 
                            &mut mem_counters as *mut PROCESS_MEMORY_COUNTERS as *mut _, 
                            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32
                        ) != BOOL(0) {
                            mem_counters.working_set_size
                        } else {
                            0
                        }
                    };

                    results.total_memory_after += after_memory;

                    // Calculer la mémoire libérée
                    let freed_memory = if before_memory > after_memory {
                        before_memory - after_memory
                    } else {
                        0
                    };

                    if freed_memory > 0 {
                        results.cleaned_count += 1;
                        results.processes.push(CleanedProcess {
                            name: process_name,
                            memory_freed: freed_memory,
                        });
                    }
                }

                unsafe { let _ = CloseHandle(handle); }
            }
        }
    }

    results.is_completed = true;
    results.end_time = Some(Local::now());
    Ok(results)
}

// Nouvelle fonction pour obtenir les informations sur la mémoire système
fn get_system_memory_info() -> (usize, usize) {
    // Utiliser winapi pour obtenir les informations sur la mémoire
    use std::mem::size_of;
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut mem_info = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    unsafe {
        if GlobalMemoryStatusEx(&mut mem_info) != 0 {
            return (mem_info.ullTotalPhys as usize, mem_info.ullAvailPhys as usize);
        }
    }

    (0, 0) // En cas d'échec, retourner des valeurs par défaut
}

// Formater la taille en unités lisibles
fn format_size(size: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size as f64;
    
    if size < KB {
        format!("{:.0} B", size)
    } else if size < MB {
        format!("{:.2} KB", size / KB)
    } else if size < GB {
        format!("{:.2} MB", size / MB)
    } else {
        format!("{:.2} GB", size / GB)
    }
}

// Remplacer la fonction qui charge l'icône par utilisation du logo intégré
fn create_app_icon() -> IconData {
    // Utiliser le logo intégré
    if let Ok(image) = load_from_memory(LOGO_BYTES) {
        let width = image.width() as u32;
        let height = image.height() as u32;
        let rgba = image.to_rgba8().into_raw();
        return IconData {
            rgba,
            width,
            height,
        };
    }
    
    // Si le chargement échoue, créer une icône par défaut
    // Créer une image de 32x32 pixels
    let width = 32;
    let height = 32;
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);

    // Générer un dégradé bleu avec un motif rappelant la mémoire
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Créer un dégradé du centre vers l'extérieur
        let dx = (x as f32 / width as f32 - 0.5) * 2.0;
        let dy = (y as f32 / height as f32 - 0.5) * 2.0;
        let distance = (dx * dx + dy * dy).sqrt();
        
        // Couleur de base bleu
        let mut r = 30;
        let mut g = 144;
        let mut b = 255;
        
        // Ajuster l'intensité en fonction de la distance
        let intensity = (1.0 - distance).max(0.0);
        r = (r as f32 * intensity) as u8;
        g = (g as f32 * intensity) as u8;
        b = (b as f32 * intensity) as u8;
        
        // Ajouter un motif de "circuit" pour représenter la mémoire
        if (x % 8 == 0 || y % 8 == 0) && distance < 0.9 {
            r = (r as f32 * 1.2).min(255.0) as u8;
            g = (g as f32 * 1.2).min(255.0) as u8;
            b = (b as f32 * 1.2).min(255.0) as u8;
        }
        
        *pixel = Rgba([r, g, b, 255]);
    }

    // Convertir l'image en RGBA pour egui
    let rgba = img.into_raw();
    
    IconData {
        rgba,
        width,
        height,
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(400.0, 500.0)),
        resizable: true,
        icon_data: Some(create_app_icon()),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Simple RAM Cleaner",
        options,
        Box::new(|cc| Box::new(CleanRamApp::new(cc))),
    )
} 