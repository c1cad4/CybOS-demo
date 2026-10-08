use crate::CybOs;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

#[derive(Clone, Copy)]
struct FarmSector {
    title: &'static str,
    subtitle: &'static str,
    symbol: &'static str,
    rect: [f32; 4],
}

const SECTORS: [FarmSector; 11] = [
    FarmSector { title: "ACORN LAND", subtitle: "oaks · seeds · shade", symbol: "ρ", rect: [0.03,0.07,0.25,0.26] },
    FarmSector { title: "BEE LAND", subtitle: "bees · honey · cameras", symbol: "β", rect: [0.29,0.07,0.27,0.26] },
    FarmSector { title: "GARDEN", subtitle: "cucumbers · corn · harvest", symbol: "∇", rect: [0.58,0.07,0.39,0.31] },
    FarmSector { title: "ORCHARD", subtitle: "apples · pears", symbol: "◌", rect: [0.30,0.36,0.25,0.18] },
    FarmSector { title: "FOREST", subtitle: "seasonal forest", symbol: "ξ", rect: [0.04,0.55,0.27,0.37] },
    FarmSector { title: "LAKE", subtitle: "fish · crayfish · birds", symbol: "≈", rect: [0.58,0.52,0.39,0.40] },
    FarmSector { title: "CHICKEN", subtitle: "feed · eggs · camera", symbol: "χ", rect: [0.04,0.80,0.15,0.18] },
    FarmSector { title: "TURKEY HOUSE", subtitle: "10 turkeys · camera", symbol: "τ", rect: [0.20,0.80,0.15,0.18] },
    FarmSector { title: "GOAT HOUSE", subtitle: "milk · feed · camera", symbol: "γ", rect: [0.36,0.80,0.15,0.18] },
    FarmSector { title: "DUCK HOUSE", subtitle: "60 ducks · camera", symbol: "δ", rect: [0.65,0.80,0.15,0.18] },
    FarmSector { title: "GOOSE HOUSE", subtitle: "40 geese · camera", symbol: "ω", rect: [0.81,0.80,0.15,0.18] },
];

const SEASONS: [(&str, &str); 4] = [
    ("SPRING", "rain"),
    ("SUMMER", "clear"),
    ("AUTUMN", "leaves"),
    ("WINTER", "snow"),
];

impl CybOs {
    pub(crate) fn farm(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let cyan = Color32::from_rgb(85, 233, 255);
        let ink = Color32::from_rgb(223, 255, 238);
        let line = Color32::from_rgba_unmultiplied(73, 255, 157, 110);
        let panel = Color32::from_rgba_unmultiplied(3, 12, 8, 242);

        self.farm_background(ui, line);

        // HTML topbar: brand + the five web controls.
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(2, 8, 5, 225))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CICADAFARM").size(15.0).strong().color(neon));
                    ui.label(RichText::new("living farm · explore · participate").size(8.0).color(Color32::from_gray(125)));
                    ui.add_space(12.0);

                    for (label, action) in [
                        ("JOURNAL", 0_u8),
                        ("KNOWLEDGE", 1),
                        ("CAMERAS", 2),
                        ("HIVES", 3),
                        ("TOKEN", 4),
                    ] {
                        let r = ui.add(
                            egui::Button::new(RichText::new(label).size(9.0).strong().color(ink))
                                .fill(panel)
                                .stroke(Stroke::new(1.0, line))
                                .min_size(Vec2::new(72.0, 27.0)),
                        );
                        if r.clicked() {
                            self.farm_action(action);
                        }
                    }
                });
            });

        ui.add_space(7.0);

        // Web map translated to a native painter so the spatial structure remains intact.
        let map_h = ui.available_height().max(520.0).min(650.0);
        let (map_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), map_h), egui::Sense::hover());
        let painter = ui.painter_at(map_rect);
        painter.rect_filled(map_rect, 18.0, Color32::from_rgb(2, 25, 15));
        painter.rect_stroke(map_rect, 18.0, Stroke::new(1.0, line), egui::StrokeKind::Outside);

        // Grid + roads.
        let step = 30.0;
        let mut x = map_rect.left();
        while x < map_rect.right() {
            painter.line_segment([egui::pos2(x,map_rect.top()),egui::pos2(x,map_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(73,255,157,10)));
            x += step;
        }
        let mut y = map_rect.top();
        while y < map_rect.bottom() {
            painter.line_segment([egui::pos2(map_rect.left(),y),egui::pos2(map_rect.right(),y)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(73,255,157,10)));
            y += step;
        }
        painter.line_segment(
            [egui::pos2(map_rect.left()+map_rect.width()*0.05,map_rect.center().y),
             egui::pos2(map_rect.right()-map_rect.width()*0.05,map_rect.center().y-18.0)],
            Stroke::new(18.0, Color32::from_rgb(20,70,43)));
        painter.line_segment(
            [egui::pos2(map_rect.center().x,map_rect.top()+20.0),
             egui::pos2(map_rect.center().x+18.0,map_rect.bottom()-20.0)],
            Stroke::new(18.0, Color32::from_rgb(20,70,43)));

        for (i, sector) in SECTORS.iter().enumerate() {
            let r = egui::Rect::from_min_max(
                egui::pos2(map_rect.left()+sector.rect[0]*map_rect.width(), map_rect.top()+sector.rect[1]*map_rect.height()),
                egui::pos2(map_rect.left()+sector.rect[2]*map_rect.width(), map_rect.top()+sector.rect[3]*map_rect.height()),
            );
            let response = ui.interact(r, ui.id().with(("farm-sector", i)), egui::Sense::click());
            let border = if response.hovered() { neon } else { line };
            painter.rect_filled(r, 13.0, if sector.title == "LAKE" { Color32::from_rgb(0,55,72) } else { Color32::from_rgba_unmultiplied(4,23,14,225) });
            painter.rect_stroke(r, 13.0, Stroke::new(if response.hovered(){2.0}else{1.0}, border), egui::StrokeKind::Outside);
            painter.text(r.left_top()+Vec2::new(9.0,8.0), egui::Align2::LEFT_TOP, sector.symbol, egui::FontId::proportional(20.0), neon);
            painter.text(r.left_top()+Vec2::new(32.0,9.0), egui::Align2::LEFT_TOP, sector.title, egui::FontId::proportional(10.0), neon);
            painter.text(r.left_top()+Vec2::new(9.0,31.0), egui::Align2::LEFT_TOP, sector.subtitle, egui::FontId::proportional(8.0), Color32::from_rgb(150,210,175));
            if response.clicked() {
                self.open_farm_panel(sector.title);
            }
        }

        // RobotCYB: the HTML image becomes the same conceptual clickable node.
        let robot_center = map_rect.center() + Vec2::new(map_rect.width()*0.01, -map_rect.height()*0.07);
        painter.circle_filled(robot_center, 38.0, Color32::from_rgb(2,35,31));
        painter.circle_stroke(robot_center, 40.0, Stroke::new(2.0, cyan));
        painter.text(robot_center, egui::Align2::CENTER_CENTER, "◈", egui::FontId::proportional(30.0), cyan);
        painter.text(robot_center + Vec2::new(0.0,-52.0), egui::Align2::CENTER_CENTER, "ROBOTCYB", egui::FontId::proportional(8.0), ink);
        let rr=egui::Rect::from_center_size(robot_center,Vec2::splat(84.0));
        if ui.interact(rr,ui.id().with("farm-robot"),egui::Sense::click()).clicked(){ self.go(crate::Page::Robot); }

        // Seasonal control and stats mirror the bottom HTML controls.
        let season_name = SEASONS[self.farm_season].0;
        let weather = SEASONS[self.farm_season].1;
        let bottom = egui::Rect::from_min_max(
            egui::pos2(map_rect.left()+10.0,map_rect.bottom()-48.0),
            egui::pos2(map_rect.right()-10.0,map_rect.bottom()-8.0),
        );
        painter.rect_filled(bottom, 10.0, Color32::from_rgba_unmultiplied(2,12,8,235));
        painter.text(bottom.left_center()+Vec2::new(8.0,0.0),egui::Align2::LEFT_CENTER,
            "60+ CHICKENS   ·   2 GOATS   ·   4 HIVES   ·   3 LAKES",egui::FontId::proportional(9.0),ink);
        painter.text(bottom.right_center()-Vec2::new(90.0,0.0),egui::Align2::RIGHT_CENTER,
            format!("{} · {}", season_name, weather),egui::FontId::proportional(9.0),neon);

        if ui.add(egui::Button::new(RichText::new("CHANGE SEASON").size(9.0).color(neon))
            .fill(panel).stroke(Stroke::new(1.0,line))).clicked() {
            self.farm_season=(self.farm_season+1)%4;
            self.store.set("cicadafarm_season",&self.farm_season.to_string());
            self.notify(format!("SEASON: {}",season_name));
        }

        if let Some(panel_name)=self.farm_panel.clone() {
            self.farm_overlay(ui, &panel_name, panel, neon, cyan, ink, line);
        }
    }

    fn farm_background(&self, ui: &egui::Ui, line: Color32) {
        let rect=ui.max_rect();
        ui.painter().rect_filled(rect,0.0,Color32::from_rgb(2,5,4));
        for i in 0..90u32 {
            let x=((i.wrapping_mul(83)%1000)as f32)/1000.0;
            let y=((i.wrapping_mul(137).wrapping_add(31)%1000)as f32)/1000.0;
            ui.painter().circle_filled(
                egui::pos2(rect.left()+rect.width()*x,rect.top()+rect.height()*y),
                if i%17==0{1.2}else{0.55},
                Color32::from_rgba_unmultiplied(180,255,220, if i%17==0{100}else{45})
            );
        }
        let _=line;
    }

    fn farm_action(&mut self, action:u8) {
        match action {
            0=>self.farm_panel=Some("JOURNAL".into()),
            1=>self.farm_panel=Some("KNOWLEDGE".into()),
            2=>self.go(crate::Page::Cameras),
            3=>self.farm_panel=Some("HIVES".into()),
            4=>self.go(crate::Page::Assets),
            _=>{}
        }
    }

    fn open_farm_panel(&mut self,title:&str){ self.farm_panel=Some(title.into()); }

    fn farm_overlay(&mut self, ui:&mut egui::Ui, name:&str, panel:Color32, neon:Color32, cyan:Color32, ink:Color32, line:Color32) {
        let full=ui.max_rect();
        let overlay=egui::Rect::from_center_size(full.center(),Vec2::new(full.width().min(720.0),full.height().min(560.0)));
        ui.painter().rect_filled(full,0.0,Color32::from_rgba_unmultiplied(0,4,2,185));
        egui::Area::new(egui::Id::new("cicadafarm_overlay")).fixed_pos(overlay.min).show(ui.ctx(),|ui|{
            egui::Frame::NONE.fill(panel).stroke(Stroke::new(1.0,line)).corner_radius(16).inner_margin(15.0).show(ui,|ui|{
                ui.set_min_size(overlay.size()-Vec2::splat(30.0));
                ui.horizontal(|ui|{
                    ui.label(RichText::new(format!("{}  {}", if name=="HIVES"{"⊙"}else{"◌"},name)).size(18.0).strong().color(neon));
                    if ui.button(RichText::new("×").size(20.0).color(ink)).clicked(){self.farm_panel=None;}
                });
                ui.separator();
                match name {
                    "JOURNAL"=>self.farm_journal(ui,neon,ink),
                    "KNOWLEDGE"=>self.farm_knowledge_panel(ui,neon,ink),
                    "HIVES"=>self.farm_hives(ui,neon,ink,cyan),
                    "BEE LAND"=>self.farm_sector_panel(ui,"BEE LAND","Bees, honey, flowers and apiary observations.",neon,ink),
                    "ACORN LAND"=>self.farm_sector_panel(ui,"ACORN LAND","Old trees, seeds and shade.",neon,ink),
                    "GARDEN"=>self.farm_sector_panel(ui,"GARDEN","Seasonal vegetables and harvest.",neon,ink),
                    "ORCHARD"=>self.farm_sector_panel(ui,"ORCHARD","Apples, pears and seasonal fruit.",neon,ink),
                    "FOREST"=>self.farm_sector_panel(ui,"FOREST","The forest changes with the season.",neon,ink),
                    "LAKE"=>self.farm_sector_panel(ui,"LAKE","Fish, crayfish, ducks and geese.",neon,ink),
                    "FARM HOUSE"=>self.farm_journal(ui,neon,ink),
                    _=>self.farm_animal_panel(ui,name,neon,ink,cyan),
                }
            });
        });
    }

    fn farm_journal(&mut self,ui:&mut egui::Ui,neon:Color32,ink:Color32){
        ui.label(RichText::new("Living farm journal · local state").size(10.0).color(Color32::from_rgb(120,180,145)));
        ui.add_space(8.0);
        ui.horizontal(|ui|{
            for (v,l) in [("☀","SEASON"),("4","HIVES"),("LIVE","FARM")] {
                egui::Frame::NONE.fill(Color32::from_rgb(8,24,16)).inner_margin(10.0).corner_radius(9).show(ui,|ui|{
                    ui.label(RichText::new(v).size(16.0).strong().color(neon));
                    ui.label(RichText::new(l).size(8.0).color(ink));
                });
            }
        });
        ui.add_space(12.0);
        ui.label("Seasonality, harvest, animals and observations live here.");
        ui.label(RichText::new(format!("Current season: {}",SEASONS[self.farm_season].0)).color(neon));
    }

    fn farm_knowledge_panel(&mut self,ui:&mut egui::Ui,neon:Color32,ink:Color32){
        ui.label(RichText::new("Family knowledge archive · persistent in cybOS local database").size(10.0).color(ink));
        ui.add_space(8.0);
        ui.add(egui::TextEdit::multiline(&mut self.farm_knowledge).desired_rows(7).hint_text("Recipe, preserving method, beekeeping technique, garden method..."));
        if ui.add(egui::Button::new(RichText::new("⊕ SAVE KNOWLEDGE").strong().color(neon))).clicked(){
            let value=self.farm_knowledge.trim().to_string();
            if !value.is_empty(){
                self.store.set("cicadafarm_knowledge",&value);
                self.add_event("KNOWLEDGE","CicadaFarm knowledge saved");
                self.notify("KNOWLEDGE SAVED");
            }
        }
        ui.add_space(8.0);
        if !self.farm_knowledge.trim().is_empty() {
            egui::Frame::NONE.fill(Color32::from_rgb(8,24,16)).inner_margin(9.0).corner_radius(9).show(ui,|ui|{
                ui.label(RichText::new("SAVED").size(9.0).strong().color(neon));
                ui.label(RichText::new(self.farm_knowledge.chars().take(220).collect::<String>()).size(10.0).color(ink));
            });
        }
    }

    fn farm_hives(&mut self,ui:&mut egui::Ui,neon:Color32,ink:Color32,cyan:Color32){
        ui.label(RichText::new("Real hive → farm care → honey → observation").size(10.0).color(ink));
        ui.add_space(8.0);
        ui.horizontal(|ui|{
            for (term,label) in [("1 YEAR","RESERVE 1 YEAR"),("2 YEARS","RESERVE 2 YEARS"),("LIVE","CAMERAS")] {
                egui::Frame::NONE.fill(Color32::from_rgb(8,24,16)).inner_margin(10.0).corner_radius(9).show(ui,|ui|{
                    ui.label(RichText::new(term).size(15.0).strong().color(neon));
                    ui.label(RichText::new(label).size(8.0).color(ink));
                });
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui|{
            if ui.button(RichText::new("⊙ RESERVE 1 YEAR").color(neon)).clicked(){self.add_event("HIVE","Hive reservation request: 1 year");self.notify("HIVE REQUEST CREATED");}
            if ui.button(RichText::new("⊙ RESERVE 2 YEARS").color(neon)).clicked(){self.add_event("HIVE","Hive reservation request: 2 years");self.notify("HIVE REQUEST CREATED");}
            if ui.button(RichText::new("↗ CAMERAS").color(cyan)).clicked(){self.go(crate::Page::Cameras);}
        });
        ui.label(RichText::new("Reservation is a local request until a real payment/booking backend is connected.").size(9.0).color(Color32::from_rgb(110,160,130)));
    }

    fn farm_sector_panel(&mut self,ui:&mut egui::Ui,title:&str,text:&str,neon:Color32,ink:Color32){
        ui.label(RichText::new(text).size(12.0).color(ink));
        ui.add_space(8.0);
        ui.horizontal(|ui|{
            ui.label(RichText::new("LIVE").strong().color(neon));
            ui.label(RichText::new("SEASON").strong().color(neon));
            ui.label(RichText::new("CYB").strong().color(neon));
        });
        if title == "BEE LAND" {
            if ui.button(RichText::new("⊙ HIVE RENTAL").color(neon)).clicked(){self.farm_panel=Some("HIVES".into());}
            if ui.button(RichText::new("↗ CAMERAS").color(neon)).clicked(){self.go(crate::Page::Cameras);}
        }
    }

    fn farm_animal_panel(&mut self,ui:&mut egui::Ui,name:&str,neon:Color32,ink:Color32,cyan:Color32){
        let (count,kind)=match name {
            "CHICKEN"=>("60+","chickens"),"TURKEY HOUSE"=>("10","turkeys"),"GOAT HOUSE"=>("3","goats"),
            "DUCK HOUSE"=>("60","ducks"),"GOOSE HOUSE"=>("40","geese"),_=>("LIVE","animals")
        };
        ui.label(RichText::new(format!("{}. Farm animal room.",count)).color(ink));
        ui.add_space(8.0);
        ui.horizontal(|ui|{
            ui.label(RichText::new(format!("{} ANIMALS",count)).strong().color(neon));
            ui.label(RichText::new("LIVE CAMERA MODULE").strong().color(cyan));
        });
        ui.add_space(10.0);
        if ui.button(RichText::new("⊕ FEED").color(neon)).clicked(){self.add_event("FARM",format!("Feed action: {}",kind));self.notify(format!("FEED REQUEST: {}",kind));}
        if ui.button(RichText::new("↗ VIDEO").color(cyan)).clicked(){self.go(crate::Page::Cameras);}
        ui.label(RichText::new("Camera is a real-device integration point; cybOS does not fabricate a live stream.").size(9.0).color(Color32::from_rgb(110,160,130)));
    }
}

