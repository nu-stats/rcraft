//! The Help window (P2A.1 to P2A.3, P2A.7): R's own help pages for the
//! installed packages, a search over them, back and forward, links in "See
//! also", and a button that runs a page's examples in the session.
//!
//! The text comes from R (see `rcodegen::help`): a hidden query asks the
//! session for the page, so nothing is added to Results, the log or the
//! history until the user runs an example. Works offline.
//!
//! RCraft's own manual (P2A.6, see `manual`) shows here too: Help > RCraft
//! manual and `help rcraft` open its contents without asking R, and a search
//! ranks the manual's pages and R's help pages together. With no R, a search
//! answers from the manual alone.

use std::time::Duration;

use eframe::egui::{self, RichText, TextStyle};
use rcraft_engine::{Engine, QueryId};
use rcraft_rcodegen::dialog_help::DialogHelp;
use rcraft_rcodegen::help::{
    page_code, page_of, parse_page, parse_search, search_code, HelpCommand, Page, PageAnswer,
};
use rcraft_rcodegen::help_docs::{
    extension, file_text_code, file_url, is_base_package, kind_of, manual_url, news_code,
    package_code, parse_package, parse_text, parse_vignettes, vignettes_code, FileKind,
    PackageAnswer, PackageDoc, Vignette,
};
use rcraft_statatr::differences::{self, Guide};

use crate::look::Palette;
use crate::manual::{self, Block, Found, Span, Target};

/// Most help topics listed on a package's page at once.
const TOPICS_SHOWN: usize = 300;

/// What the window asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpOut {
    /// Run these examples in the session, as typed.
    Run(String),
    /// The topic is not in any installed package: look for a package.
    FindPackages(String),
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Content {
    Page(Box<Page>),
    NotFound {
        topic: String,
        suggestions: Vec<String>,
    },
    Results {
        words: String,
        /// The manual's pages and R's help pages that fit, in one ranking.
        found: Vec<Found>,
        /// False when there was no R to ask, so only the manual was searched.
        asked_r: bool,
    },
    /// A page of RCraft's own manual, by its name.
    Manual(&'static str),
    Message(String),
    /// The help page of one of RCraft's own dialogs.
    Dialog(Box<DialogHelp>),
    /// Where R and Stata differ for one command (P2B.8), or the general page.
    Stata(&'static Guide),
    /// A package's own page.
    Package(Box<PackageDoc>),
    /// This package is not installed.
    NoPackage(String),
    /// The vignettes of one package, or of all.
    Vignettes {
        package: Option<String>,
        list: Vec<Vignette>,
    },
    /// A NEWS file or a vignette's source or code. `runnable` is for R code.
    Text {
        title: String,
        text: String,
        runnable: bool,
    },
}

#[derive(Default)]
pub struct HelpWindow {
    pub open: bool,
    /// The search box.
    query: String,
    /// What was shown, oldest first, and which one is showing.
    shown: Vec<Content>,
    at: usize,
    /// A request not yet sent to R (R is not ready, or just asked for).
    queued: Option<HelpCommand>,
    /// A request R is answering.
    pending: Option<(QueryId, HelpCommand)>,
    /// Ask for the search box to take the keyboard.
    focus_query: bool,
    /// The CRAN mirror the Packages window uses, for a reference manual's address.
    mirror: String,
    /// Whether the package shown is on CRAN, when the package list says (it may
    /// not be loaded).
    on_cran: Option<bool>,
}

impl HelpWindow {
    /// Opens the window on a request: a page, or a search.
    pub fn request(&mut self, command: HelpCommand) {
        self.open = true;
        // `help rcraft` and `?rcraft` are the manual's contents, answered here, not by R
        if matches!(&command, HelpCommand::Page { topic, package: None } if topic == "rcraft") {
            self.show_manual(None);
            return;
        }
        if let HelpCommand::Search { words } = &command {
            self.query = words.clone();
        }
        self.queued = Some(command);
    }

    /// Opens the window on a page of RCraft's manual (its contents when none,
    /// or when the name is not a page). Nothing is asked of R.
    pub fn show_manual(&mut self, slug: Option<&str>) {
        self.open = true;
        let page = slug
            .and_then(manual::page)
            .map_or(manual::CONTENTS, |p| p.slug);
        self.push(Content::Manual(page));
    }

    /// The manual page showing, if one is (tests).
    #[cfg(test)]
    pub fn shown_manual(&self) -> Option<&'static str> {
        match self.current() {
            Some(Content::Manual(slug)) => Some(slug),
            _ => None,
        }
    }

    /// Opens the window on the help page of a dialog. Nothing is asked of R.
    pub fn show_dialog(&mut self, page: DialogHelp) {
        self.open = true;
        self.push(Content::Dialog(Box::new(page)));
    }

    /// Opens the window on the differences guide's page for the command a
    /// Stata line runs (the general page when it has none). Nothing is asked of R.
    pub fn show_stata(&mut self, line: &str) {
        self.open = true;
        let page = differences::for_line(line).or_else(|| differences::guide("general"));
        if let Some(page) = page {
            self.push(Content::Stata(page));
        }
    }

    /// The command whose page of the differences guide is showing, if one is.
    #[cfg(test)]
    pub fn shown_stata(&self) -> Option<&'static str> {
        match self.current() {
            Some(Content::Stata(page)) => Some(page.command),
            _ => None,
        }
    }

    /// The vignettes listed, if a list is showing (tests).
    #[cfg(test)]
    pub fn shown_vignettes(&self) -> Option<&[Vignette]> {
        match self.current() {
            Some(Content::Vignettes { list, .. }) => Some(list),
            _ => None,
        }
    }

    /// The text showing (a vignette's code or source, a NEWS file), if one is (tests).
    #[cfg(test)]
    pub fn shown_text(&self) -> Option<&str> {
        match self.current() {
            Some(Content::Text { text, .. }) => Some(text),
            _ => None,
        }
    }

    /// Opens the window at the search box.
    pub fn open_search(&mut self) {
        self.open = true;
        self.focus_query = true;
    }

    /// The package whose page is showing, if one is.
    pub fn shown_package(&self) -> Option<&str> {
        match self.current() {
            Some(Content::Package(doc)) => Some(doc.package.as_str()),
            _ => None,
        }
    }

    /// What the application knows about CRAN: the mirror, and whether the
    /// package on show is there (`None` when the package list is not loaded).
    pub fn set_cran(&mut self, mirror: &str, on_cran: Option<bool>) {
        self.mirror = mirror.to_string();
        self.on_cran = on_cran;
    }

    /// True while a request waits for R or R is answering it.
    pub fn is_busy(&self) -> bool {
        self.queued.is_some() || self.pending.is_some()
    }

    fn current(&self) -> Option<&Content> {
        self.shown.get(self.at)
    }

    fn push(&mut self, content: Content) {
        self.shown
            .truncate(self.at + usize::from(!self.shown.is_empty()));
        self.shown.push(content);
        self.at = self.shown.len() - 1;
    }

    /// Takes R's answer to a request.
    fn apply_answer(&mut self, request: &HelpCommand, ok: bool, text: &str) {
        let content = if !ok {
            Content::Message(format!("R could not answer that: {}", text.trim()))
        } else {
            match request {
                HelpCommand::Page { topic, .. } => match parse_page(text) {
                    Some(PageAnswer::Found(page)) => Content::Page(page),
                    Some(PageAnswer::NotFound { suggestions }) => Content::NotFound {
                        topic: topic.clone(),
                        suggestions,
                    },
                    None => Content::Message("R's answer could not be read.".into()),
                },
                HelpCommand::Search { words } => match parse_search(words, text) {
                    Some(hits) => Content::Results {
                        words: words.clone(),
                        found: manual::rank_together(words, manual::search(words), hits),
                        asked_r: true,
                    },
                    None => Content::Message("The search did not finish.".into()),
                },
                HelpCommand::Package { package } => match parse_package(text) {
                    Some(PackageAnswer::Found(doc)) => Content::Package(doc),
                    Some(PackageAnswer::NotInstalled) => Content::NoPackage(package.clone()),
                    None => Content::Message("R's answer could not be read.".into()),
                },
                HelpCommand::Vignettes { package } => match parse_vignettes(text) {
                    Some(list) => Content::Vignettes {
                        package: package.clone(),
                        list,
                    },
                    None => Content::Message("R's answer could not be read.".into()),
                },
                HelpCommand::News { package } => match parse_text(text) {
                    Ok(Some(doc)) => Content::Text {
                        title: doc.title,
                        text: doc.text,
                        runnable: false,
                    },
                    Ok(None) => Content::Message(format!("{package} has no NEWS file.")),
                    Err(()) => Content::Message("R's answer could not be read.".into()),
                },
                HelpCommand::File { path } => match parse_text(text) {
                    Ok(Some(doc)) => Content::Text {
                        title: doc.title,
                        text: doc.text,
                        runnable: extension(path) == "r",
                    },
                    Ok(None) => Content::Message("That file could not be shown.".into()),
                    Err(()) => Content::Message("R's answer could not be read.".into()),
                },
            }
        };
        self.push(content);
    }

    /// Sends a queued request to R and takes an answer that has come. With no
    /// R, a search is answered from the manual alone.
    fn service(&mut self, engine: Option<&mut Engine>) {
        let Some(engine) = engine else {
            if let Some(HelpCommand::Search { words }) = &self.queued {
                let found = manual::rank_together(words, manual::search(words), Vec::new());
                let words = words.clone();
                self.queued = None;
                self.push(Content::Results {
                    words,
                    found,
                    asked_r: false,
                });
            }
            return;
        };
        if let Some((id, request)) = self.pending.clone() {
            if let Some(r) = engine.query_result(id) {
                let (ok, text) = (r.ok, r.text.clone());
                self.pending = None;
                self.apply_answer(&request, ok, &text);
            }
        }
        if self.pending.is_none() {
            if let Some(request) = self.queued.take() {
                let code = match &request {
                    HelpCommand::Page { topic, package } => page_code(topic, package.as_deref()),
                    HelpCommand::Search { words } => search_code(words),
                    HelpCommand::Package { package } => package_code(package),
                    HelpCommand::Vignettes { package } => vignettes_code(package.as_deref()),
                    HelpCommand::News { package } => news_code(package),
                    HelpCommand::File { path } => file_text_code(path),
                };
                self.pending = Some((engine.query(&code), request));
            }
        }
    }

    /// Draws the window. Returns what to do outside it.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        engine: Option<&mut Engine>,
    ) -> Option<HelpOut> {
        if !self.open {
            return None;
        }
        self.service(engine);
        if self.pending.is_some() || self.queued.is_some() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        let mut out = None;
        let mut next: Option<HelpCommand> = None;
        let mut guide: Option<&'static Guide> = None;
        let mut page: Option<&'static str> = None;
        let mut open = self.open;
        egui::Window::new("Help")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            // never taller than the window, so its last lines and buttons stay reachable (SW23)
            .default_size([
                700.0_f32.min(ctx.screen_rect().width() - 24.0),
                560.0_f32.min(ctx.screen_rect().height() - 90.0).max(240.0),
            ])
            .max_height((ctx.screen_rect().height() - 60.0).max(240.0))
            .default_pos(egui::pos2(240.0, 60.0))
            .show(ctx, |ui| {
                self.toolbar(ui, &mut next);
                ui.separator();
                if self.pending.is_some() || self.queued.is_some() {
                    ui.label(RichText::new("Asking R…").color(pal.dim));
                }
                self.body(ui, pal, ctx, &mut next, &mut guide, &mut page, &mut out);
            });
        self.open = open;
        if let Some(page) = guide {
            self.push(Content::Stata(page));
        }
        if let Some(slug) = page {
            self.push(Content::Manual(slug));
        }
        if let Some(command) = next {
            self.queued = Some(command);
        }
        out
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, next: &mut Option<HelpCommand>) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.at > 0, egui::Button::new("◀ Back"))
                .clicked()
            {
                self.at -= 1;
            }
            if ui
                .add_enabled(
                    self.at + 1 < self.shown.len(),
                    egui::Button::new("Forward ▶"),
                )
                .clicked()
            {
                self.at += 1;
            }
            ui.separator();
            let field = egui::TextEdit::singleline(&mut self.query)
                .hint_text("Topic or words, e.g. lm, or linear regression")
                .desired_width(ui.available_width() - 150.0);
            let response = ui.add(field);
            if self.focus_query {
                response.request_focus();
                self.focus_query = false;
            }
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let words = self.query.trim().to_string();
            if (ui
                .add_enabled(!words.is_empty(), egui::Button::new("Search"))
                .clicked()
                || enter)
                && !words.is_empty()
            {
                *next = Some(HelpCommand::Search { words });
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn body(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        next: &mut Option<HelpCommand>,
        guide: &mut Option<&'static Guide>,
        manual_page: &mut Option<&'static str>,
        out: &mut Option<HelpOut>,
    ) {
        match self.current() {
            None => {
                ui.add_space(8.0);
                if ui.link("RCraft manual").clicked() {
                    *manual_page = Some(manual::CONTENTS);
                }
                ui.label("Type a topic or some words above. For example: mean, lm, or linear regression.");
                ui.label(
                    RichText::new(
                        "This is R's own help for the packages installed here; it works with no network.\n\
                         In the Command window: ?mean, help(mean), help mean, ??words or help search words.",
                    )
                    .small()
                    .color(pal.dim),
                );
            }
            Some(Content::Message(text)) => {
                ui.label(RichText::new(text).color(pal.warning));
            }
            Some(Content::NotFound { topic, suggestions }) => {
                ui.label(RichText::new(format!("No help page for \"{topic}\".")).strong());
                if !suggestions.is_empty() {
                    ui.label("Names that contain it:");
                    ui.horizontal_wrapped(|ui| {
                        for s in suggestions {
                            if ui.link(s).clicked() {
                                *next = Some(HelpCommand::Page {
                                    topic: s.clone(),
                                    package: None,
                                });
                            }
                        }
                    });
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "If it belongs to a package that is not installed, find the package first.",
                    )
                    .small()
                    .color(pal.dim),
                );
                if ui.button("Find a package for this…").clicked() {
                    *out = Some(HelpOut::FindPackages(topic.clone()));
                }
                if ui.button(format!("Search help for \"{topic}\"")).clicked() {
                    *next = Some(HelpCommand::Search {
                        words: topic.clone(),
                    });
                }
            }
            Some(Content::Results {
                words,
                found,
                asked_r,
            }) => {
                ui.label(
                    RichText::new(format!("{} help pages for \"{words}\"", found.len())).strong(),
                );
                if !asked_r {
                    ui.label(
                        RichText::new("R is not running, so only RCraft's manual was searched.")
                            .color(pal.dim),
                    );
                } else if !found.iter().any(|f| matches!(f, Found::R(_))) {
                    ui.label("Nothing found in the installed packages.");
                    if ui.button("Find a package for this…").clicked() {
                        *out = Some(HelpOut::FindPackages(words.clone()));
                    }
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for f in found {
                        ui.horizontal_wrapped(|ui| match f {
                            Found::Manual(h) => {
                                if ui.link(RichText::new(h.title).strong()).clicked() {
                                    *manual_page = Some(h.slug);
                                }
                                ui.label(RichText::new("(RCraft manual)").color(pal.dim));
                            }
                            Found::R(h) => {
                                if ui.link(RichText::new(&h.topic).strong()).clicked() {
                                    *next = Some(HelpCommand::Page {
                                        topic: h.topic.clone(),
                                        package: Some(h.package.clone()),
                                    });
                                }
                                ui.label(RichText::new(format!("({})", h.package)).color(pal.dim));
                                ui.label(&h.title);
                            }
                        });
                    }
                });
            }
            Some(Content::Page(page)) => self.page(ui, pal, ctx, page, next, out),
            Some(Content::Dialog(page)) => self.dialog_page(ui, pal, ctx, page, next),
            Some(Content::Stata(page)) => stata_page(ui, pal, page, guide),
            Some(Content::Manual(slug)) => manual_view(ui, pal, slug, next, manual_page),
            Some(Content::Package(doc)) => self.package_page(ui, pal, ctx, doc, next),
            Some(Content::NoPackage(package)) => {
                ui.label(
                    RichText::new(format!("The package \"{package}\" is not installed here."))
                        .strong(),
                );
                if ui.button("Find it in Packages…").clicked() {
                    *out = Some(HelpOut::FindPackages(package.clone()));
                }
            }
            Some(Content::Vignettes { package, list }) => {
                self.vignettes_page(ui, pal, ctx, package.as_deref(), list, next);
            }
            Some(Content::Text {
                title,
                text,
                runnable,
            }) => self.text_page(ui, pal, ctx, title, text, *runnable, out),
        }
    }

    /// One vignette: its title and a button for each file R has for it. A
    /// rendered document is opened by the system (only on this click); R code
    /// and source are shown here, the code with a button to run it.
    fn vignette_row(
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        v: &Vignette,
        show_package: bool,
        next: &mut Option<HelpCommand>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&v.title).strong());
            if show_package {
                ui.label(RichText::new(format!("({})", v.package)).color(pal.dim));
            }
            for path in &v.files {
                let Some(kind) = kind_of(path) else { continue };
                let label = match kind {
                    FileKind::Document => format!("Open {}", extension(path).to_uppercase()),
                    other => other.label().to_string(),
                };
                if ui.small_button(label).clicked() {
                    match kind {
                        FileKind::Document => ctx.open_url(egui::OpenUrl::new_tab(file_url(path))),
                        FileKind::Code | FileKind::Source => {
                            *next = Some(HelpCommand::File { path: path.clone() });
                        }
                    }
                }
            }
            if v.files.is_empty() {
                ui.label(RichText::new("no file to open").small().color(pal.dim));
            }
        });
    }

    /// A package's page: its description, vignettes, NEWS and help index.
    fn package_page(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        doc: &PackageDoc,
        next: &mut Option<HelpCommand>,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&doc.package).strong().size(17.0));
                    if ui
                        .add_enabled(
                            !doc.vignettes.is_empty(),
                            egui::Button::new(format!("Vignettes ({})", doc.vignettes.len())),
                        )
                        .clicked()
                    {
                        *next = Some(HelpCommand::Vignettes {
                            package: Some(doc.package.clone()),
                        });
                    }
                    let base = is_base_package(&doc.description);
                    let on_cran = base || self.on_cran != Some(false);
                    let manual = ui
                        .add_enabled(on_cran, egui::Button::new("Reference manual (PDF)"))
                        .on_hover_text(if base {
                            "This package comes with R: opens R's full reference manual (a PDF on CRAN) in your browser"
                        } else {
                            "Opens the package's reference manual (a PDF on CRAN) in your browser. RCraft sends nothing."
                        })
                        .on_disabled_hover_text("This package is not on CRAN, so it has no CRAN manual");
                    if manual.clicked() && !self.mirror.is_empty() {
                        ctx.open_url(egui::OpenUrl::new_tab(manual_url(
                            &self.mirror,
                            &doc.package,
                            base,
                        )));
                    }
                    if ui
                        .add_enabled(doc.has_news, egui::Button::new("News"))
                        .on_disabled_hover_text("This package has no NEWS file")
                        .clicked()
                    {
                        *next = Some(HelpCommand::News {
                            package: doc.package.clone(),
                        });
                    }
                });
                egui::CollapsingHeader::new("About this package")
                    .id_salt(("help-about", &doc.package))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(&doc.description).monospace().small())
                                .selectable(true),
                        );
                    });
                if !doc.vignettes.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Vignettes").strong().color(pal.dim));
                    for v in &doc.vignettes {
                        Self::vignette_row(ui, pal, ctx, v, false, next);
                    }
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("Help topics ({})", doc.topics.len()))
                        .strong()
                        .color(pal.dim),
                );
                let id = ui.make_persistent_id(("help-topic-filter", &doc.package));
                let mut filter: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
                ui.add(
                    egui::TextEdit::singleline(&mut filter)
                        .hint_text("Narrow the list")
                        .desired_width(220.0),
                );
                ui.data_mut(|d| d.insert_temp(id, filter.clone()));
                let needle = filter.trim().to_lowercase();
                let shown: Vec<_> = doc
                    .topics
                    .iter()
                    .filter(|t| {
                        needle.is_empty()
                            || t.name.to_lowercase().contains(&needle)
                            || t.title.to_lowercase().contains(&needle)
                    })
                    .collect();
                for t in shown.iter().take(TOPICS_SHOWN) {
                    ui.horizontal_wrapped(|ui| {
                        if ui.link(RichText::new(&t.name).strong()).clicked() {
                            *next = Some(HelpCommand::Page {
                                topic: t.name.clone(),
                                package: Some(doc.package.clone()),
                            });
                        }
                        ui.label(&t.title);
                    });
                }
                if shown.len() > TOPICS_SHOWN {
                    ui.label(
                        RichText::new(format!(
                            "{} more; type in the box to narrow the list.",
                            shown.len() - TOPICS_SHOWN
                        ))
                        .color(pal.dim),
                    );
                }
                if shown.is_empty() {
                    ui.label(RichText::new("No topic matches.").color(pal.dim));
                }
            });
    }

    /// The vignettes of one package, or of every installed one by package.
    fn vignettes_page(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        package: Option<&str>,
        list: &[Vignette],
        next: &mut Option<HelpCommand>,
    ) {
        let heading = match package {
            Some(p) => format!("Vignettes of {p}"),
            None => format!("Vignettes of the installed packages ({})", list.len()),
        };
        ui.label(RichText::new(heading).strong().size(15.0));
        if list.is_empty() {
            ui.label("None found. Vignettes are long-form guides some packages ship with.");
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut last: Option<&str> = None;
                for v in list {
                    if package.is_none() && last != Some(v.package.as_str()) {
                        last = Some(v.package.as_str());
                        ui.add_space(6.0);
                        if ui.link(RichText::new(&v.package).strong()).clicked() {
                            *next = Some(HelpCommand::Package {
                                package: v.package.clone(),
                            });
                        }
                    }
                    Self::vignette_row(ui, pal, ctx, v, false, next);
                }
            });
    }

    /// A text: a NEWS file, or a vignette's source or R code.
    #[allow(clippy::too_many_arguments)]
    fn text_page(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        title: &str,
        text: &str,
        runnable: bool,
        out: &mut Option<HelpOut>,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().size(15.0));
            if runnable
                && ui
                    .add(
                        egui::Button::new(RichText::new("Run code").strong().color(pal.on_accent))
                            .fill(pal.accent),
                    )
                    .on_hover_text("Runs this R code in the session; the output shows in Results")
                    .clicked()
            {
                *out = Some(HelpOut::Run(text.to_string()));
            }
            if ui.button("Copy").clicked() {
                ctx.copy_text(text.to_string());
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(text).text_style(TextStyle::Monospace))
                        .selectable(true),
                );
            });
    }

    /// The help page of one of RCraft's dialogs: what it does, its options,
    /// the packages it needs, an example of the R it writes, and links to R's
    /// help for the functions that R calls.
    fn dialog_page(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        page: &DialogHelp,
        next: &mut Option<HelpCommand>,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new(&page.title).strong().size(17.0));
                ui.label(RichText::new(&page.menu).small().color(pal.dim));
                ui.add_space(4.0);
                ui.label(&page.about);
                for note in &page.notes {
                    ui.add_space(4.0);
                    ui.label(note);
                }
                if !page.options.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("What you can set").strong().color(pal.dim));
                    egui::Grid::new("dialog-help-options")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for o in &page.options {
                                ui.label(RichText::new(&o.label).strong());
                                ui.label(&o.text);
                                ui.end_row();
                            }
                        });
                }
                if !page.needs.is_empty() || page.other_style.is_some() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("R packages").strong().color(pal.dim));
                    if page.needs.is_empty() {
                        ui.label("Needs no package beyond R itself.");
                    } else {
                        ui.label(format!("Needs {}.", page.needs.join(", ")));
                    }
                    if let Some((style, packages)) = &page.other_style {
                        if packages.is_empty() {
                            ui.label(format!("{style} needs no package either."));
                        } else {
                            ui.label(format!("{style} needs {}.", packages.join(", ")));
                        }
                    }
                    ui.label(
                        RichText::new(
                            "A missing package is offered for install in the dialog; nothing installs without your yes.",
                        )
                        .small()
                        .color(pal.dim),
                    );
                }
                if let Some(example) = &page.example {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("The R it writes (an example)")
                            .strong()
                            .color(pal.dim),
                    );
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(egui::Label::new(RichText::new(example).monospace()).selectable(true));
                    });
                    if ui.small_button("Copy").clicked() {
                        ctx.copy_text(example.clone());
                    }
                    ui.label(
                        RichText::new("The dialog shows the exact R for your choices before it runs, and it goes into the Results, log and script.")
                            .small()
                            .color(pal.dim),
                    );
                }
                if !page.functions.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("R functions it uses (click for R's help)").strong().color(pal.dim));
                    ui.horizontal_wrapped(|ui| {
                        for f in &page.functions {
                            if ui.link(f).clicked() {
                                *next = page_of(f);
                            }
                        }
                    });
                }
            });
    }

    fn page(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        page: &Page,
        next: &mut Option<HelpCommand>,
        out: &mut Option<HelpOut>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&page.topic).strong().size(17.0));
            ui.label(RichText::new(format!("{{{}}}", page.package)).color(pal.dim));
            if ui.small_button("Package page").clicked() {
                *next = Some(HelpCommand::Package {
                    package: page.package.clone(),
                });
            }
            ui.label(&page.title);
        });
        if !page.also_in.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Also in:").small().color(pal.dim));
                for p in &page.also_in {
                    if ui.small_button(p).clicked() {
                        *next = Some(HelpCommand::Page {
                            topic: page.topic.clone(),
                            package: Some(p.clone()),
                        });
                    }
                }
            });
        }
        if !page.links.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("See also:").small().color(pal.dim));
                for l in &page.links {
                    if ui.link(l).clicked() {
                        *next = Some(HelpCommand::Page {
                            topic: l.clone(),
                            package: None,
                        });
                    }
                }
            });
        }
        if !page.examples.is_empty() {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Run examples").strong().color(pal.on_accent),
                        )
                        .fill(pal.accent),
                    )
                    .on_hover_text(
                        "Runs the page's examples in the R session; they show in Results",
                    )
                    .clicked()
                {
                    *out = Some(HelpOut::Run(page.examples.clone()));
                }
                if ui.button("Copy examples").clicked() {
                    ctx.copy_text(page.examples.clone());
                }
            });
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&page.text).text_style(TextStyle::Monospace))
                        .selectable(true),
                );
            });
    }
}

/// One line of a manual page: text, bold, code and links, wrapping together. A
/// link to a page sets `page`; a `help:` link asks for R's help page. A link
/// that names nothing is shown as plain text.
fn spans_row(
    ui: &mut egui::Ui,
    pal: &Palette,
    spans: &[Span],
    next: &mut Option<HelpCommand>,
    page: &mut Option<&'static str>,
) {
    ui.horizontal_wrapped(|ui| {
        // the spans' own spaces are the only ones
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            match span {
                Span::Text(t) => {
                    ui.label(t);
                }
                Span::Bold(t) => {
                    ui.label(RichText::new(t).strong());
                }
                Span::Code(t) => {
                    ui.label(RichText::new(t).monospace().color(pal.accent));
                }
                Span::Link { text, target } => match manual::resolve(target) {
                    Some(Target::Page(slug)) => {
                        if ui.link(text).clicked() {
                            *page = Some(slug);
                        }
                    }
                    Some(Target::Help(request)) => {
                        if ui.link(text).clicked() {
                            *next = Some(request);
                        }
                    }
                    None => {
                        ui.label(text);
                    }
                },
            }
        }
    });
}

/// A page of RCraft's manual: headings, paragraphs, list items, code and pictures.
fn manual_view(
    ui: &mut egui::Ui,
    pal: &Palette,
    slug: &str,
    next: &mut Option<HelpCommand>,
    page: &mut Option<&'static str>,
) {
    let Some(shown) = manual::page(slug) else {
        ui.label("That page is not in the manual.");
        return;
    };
    let top = slug != manual::CONTENTS;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if top {
                if ui.link("◀ Contents").clicked() {
                    *page = Some(manual::CONTENTS);
                }
                ui.add_space(4.0);
            }
            for block in manual::parse(shown.text) {
                match block {
                    Block::Heading(level, spans) => {
                        ui.add_space(if level == 1 { 2.0 } else { 8.0 });
                        let text = spans_text(&spans);
                        let size = if level == 1 { 19.0 } else { 15.0 };
                        ui.label(RichText::new(text).strong().size(size));
                    }
                    Block::Paragraph(spans) => {
                        ui.add_space(4.0);
                        spans_row(ui, pal, &spans, next, page);
                    }
                    Block::Item(number, spans) => {
                        ui.horizontal_top(|ui| {
                            ui.add_space(12.0);
                            ui.label(match number {
                                Some(n) => format!("{n}."),
                                None => "•".to_string(),
                            });
                            ui.vertical(|ui| spans_row(ui, pal, &spans, next, page));
                        });
                    }
                    Block::Code(text) => {
                        ui.add_space(4.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(
                                egui::Label::new(RichText::new(text).monospace()).selectable(true),
                            );
                        });
                    }
                    Block::Picture { alt, name } => {
                        ui.add_space(6.0);
                        picture_view(ui, pal, &alt, &name);
                    }
                }
            }
        });
}

/// A picture of the manual, as wide as there is room for (never wider than
/// it is), with its caption under it. A picture that cannot be read shows its
/// caption alone.
fn picture_view(ui: &mut egui::Ui, pal: &Palette, alt: &str, name: &str) {
    if let Some(texture) = manual::picture(name).and_then(|p| texture_of(ui.ctx(), p)) {
        let size = texture.size_vec2();
        // a thin border, so a picture of the window is not taken for the window
        let width = size.x.min(ui.available_width() - 2.0).max(1.0);
        let shown = egui::vec2(width, size.y * width / size.x.max(1.0));
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0_f32, pal.dim))
            .show(ui, |ui| {
                ui.add(egui::Image::new((texture.id(), shown)));
            });
    }
    ui.label(RichText::new(alt).small().italics().color(pal.dim));
}

/// The picture's texture, decoded the first time it is shown and kept.
fn texture_of(ctx: &egui::Context, p: &'static manual::Picture) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("manual-picture", p.name));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let image = picture_pixels(p)?;
    // mipmaps keep the small text legible when the picture is drawn smaller
    let options = egui::TextureOptions {
        mipmap_mode: Some(egui::TextureFilter::Linear),
        ..egui::TextureOptions::LINEAR
    };
    let texture = ctx.load_texture(format!("manual-{}", p.name), image, options);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}

/// A picture's pixels, or `None` when its PNG cannot be read.
fn picture_pixels(p: &manual::Picture) -> Option<egui::ColorImage> {
    let img = image::load_from_memory_with_format(p.png, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let size = [
        usize::try_from(img.width()).ok()?,
        usize::try_from(img.height()).ok()?,
    ];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// The words of spans without their marks (a heading).
fn spans_text(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| match s {
            Span::Text(t) | Span::Bold(t) | Span::Code(t) => t.as_str(),
            Span::Link { text, .. } => text.as_str(),
        })
        .collect()
}

/// A page of the differences guide: what the command does, then each point;
/// the general page ends with the list of the commands' pages.
fn stata_page(
    ui: &mut egui::Ui,
    pal: &Palette,
    page: &'static Guide,
    guide: &mut Option<&'static Guide>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let title = if page.command == "general" {
                "Stata and R: what differs".to_string()
            } else {
                format!("Stata and R: {}", page.command)
            };
            ui.label(RichText::new(title).strong().size(17.0));
            ui.label(
                RichText::new("Stata mode's differences guide")
                    .small()
                    .color(pal.dim),
            );
            ui.add_space(4.0);
            ui.label(page.about);
            // Verified: what the corpus checks against Stata's own results
            if let Some(files) = rcraft_statatr::verified::files(page.command) {
                ui.label(
                    RichText::new(format!(
                        "Verified: checked against Stata 19's own results ({} in the corpus)",
                        files.join(", ")
                    ))
                    .color(pal.accent),
                );
            } else if page.command != "general" {
                ui.label(
                    RichText::new("Not in the Stata corpus yet: checked by its own tests")
                        .small()
                        .color(pal.dim),
                );
            }
            for point in page.points {
                ui.add_space(6.0);
                ui.label(RichText::new(point.topic).strong());
                ui.label(point.text);
            }
            ui.add_space(10.0);
            if page.command == "general" {
                ui.label(RichText::new("Each command").strong().color(pal.dim));
                ui.horizontal_wrapped(|ui| {
                    for g in differences::GUIDES.iter().skip(1) {
                        if ui.link(g.command).on_hover_text(g.about).clicked() {
                            *guide = Some(g);
                        }
                    }
                });
            } else if ui.link("What differs for every command").clicked() {
                *guide = differences::guide("general");
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_request(topic: &str) -> HelpCommand {
        HelpCommand::Page {
            topic: topic.into(),
            package: None,
        }
    }

    const MEAN: &str = "@@PACKAGE\tbase\n@@TOPIC\tmean\n@@TITLE\tArithmetic Mean\n\
                        @@LINK\tweighted.mean\n@@TEXT\n|Arithmetic Mean\n@@END\n\
                        @@EXAMPLES\n|mean(1:3)\n@@END\n";

    /// At 560 points tall the window ran past the bottom edge (it was 560 tall itself, drawn
    /// 60 down) and its last lines were out of reach: it now fits the window (SW23).
    #[test]
    fn the_window_fits_a_short_screen() {
        let pal = Palette::new(crate::look::Accent::default(), false);
        for (width, height) in [(820.0, 560.0), (1024.0, 640.0), (1240.0, 780.0)] {
            let mut w = HelpWindow::default();
            w.show_dialog(rcraft_rcodegen::dialog_help::use_my_file());
            let ctx = egui::Context::default();
            for _ in 0..4 {
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = w.show(ctx, &pal, None);
                    },
                );
            }
            let rect = ctx
                .memory(|m| m.area_rect(egui::Id::new("Help")))
                .unwrap_or(egui::Rect::EVERYTHING);
            assert!(
                rect.right() <= width + 1.0 && rect.bottom() <= height + 1.0,
                "at {width}x{height} the Help window spans to {}x{}",
                rect.right(),
                rect.bottom()
            );
        }
    }

    /// P2A.6: every page of RCraft's manual loads and draws, every link in it
    /// goes somewhere, a search finds the Projects page first for "project", and
    /// `help rcraft` opens the contents without asking R.
    #[test]
    fn manual_pages_are_found_by_search() {
        // every page loads, has a title and reads into blocks
        for p in manual::PAGES {
            let blocks = manual::parse(p.text);
            assert!(
                matches!(blocks.first(), Some(manual::Block::Heading(1, _))),
                "{} starts with its title",
                p.slug
            );
            assert!(blocks.len() > 3, "{} has some text", p.slug);
            // every link names a page, or an R help topic that can be asked for,
            // and every picture is one of the manual's
            for block in &blocks {
                let spans = match block {
                    manual::Block::Heading(_, s)
                    | manual::Block::Paragraph(s)
                    | manual::Block::Item(_, s) => s,
                    manual::Block::Code(_) => continue,
                    manual::Block::Picture { alt, name } => {
                        assert!(
                            manual::picture(name).is_some(),
                            "{}: the picture \"{name}\" is not in the manual",
                            p.slug
                        );
                        assert!(!alt.is_empty(), "{}: {name} says what it shows", p.slug);
                        continue;
                    }
                };
                for span in spans {
                    if let Span::Link { text, target } = span {
                        assert!(
                            manual::resolve(target).is_some(),
                            "{}: the link \"{text}\" goes to \"{target}\", which is not there",
                            p.slug
                        );
                    }
                }
            }
        }
        // the contents list every other page
        let index = manual::page(manual::CONTENTS).map_or("", |p| p.text);
        for p in manual::PAGES.iter().skip(1) {
            assert!(
                index.contains(&format!("]({})", p.slug)),
                "{} is listed",
                p.slug
            );
        }
        assert!(matches!(
            manual::resolve("help:lm"),
            Some(Target::Help(HelpCommand::Page { .. }))
        ));
        assert_eq!(manual::resolve("nowhere"), None);
        // every picture is shown by a page, and reads as a picture of the window
        for pic in manual::PICTURES {
            let wanted = format!("](images/{})", pic.name);
            assert!(
                manual::PAGES.iter().any(|p| p.text.contains(&wanted)),
                "{} is shown by a page",
                pic.name
            );
            let pixels = picture_pixels(pic);
            assert!(
                pixels
                    .as_ref()
                    .is_some_and(|i| i.size[0] >= 600 && i.size[1] >= 400),
                "{} reads as a picture of the window",
                pic.name
            );
        }

        // searching "project" lists the Projects page first
        let found = manual::search("project");
        assert_eq!(
            found.first().map(|h| h.slug),
            Some("projects-and-reporting")
        );

        // `help rcraft` and `?rcraft` open the contents, and nothing is asked of R
        for typed in ["help rcraft", "?rcraft", "help(rcraft)"] {
            let mut w = HelpWindow::default();
            let command = rcraft_rcodegen::help::parse_command(typed);
            assert!(command.is_some(), "{typed} is a help request");
            if let Some(command) = command {
                w.request(command);
            }
            assert!(w.open);
            assert_eq!(w.shown_manual(), Some(manual::CONTENTS), "{typed}");
            assert!(!w.is_busy(), "{typed} asks nothing of R");
        }
        // a topic of another package is still R's
        let mut w = HelpWindow::default();
        w.request(HelpCommand::Page {
            topic: "rcraft".into(),
            package: Some("stats".into()),
        });
        assert!(w.is_busy());

        // a search ranks the manual's pages and R's hits together: R's page named
        // exactly "project" first, then the Projects page (its title starts with
        // the word), then R's page that only holds it, then pages that only
        // mention it; the manual's pages stay when R has none
        let mut w = HelpWindow::default();
        let search = HelpCommand::Search {
            words: "project".into(),
        };
        w.apply_answer(
            &search,
            true,
            "@@HIT\tbase\tproject\tA Project Of R's\n\
             @@HIT\tstats\tsubproject\tA Part Of One\n\
             @@HIT\tutils\tsomething\tMentions It\n@@DONE\n",
        );
        let order = |w: &HelpWindow| -> Vec<String> {
            match w.current() {
                Some(Content::Results { found, .. }) => found
                    .iter()
                    .map(|f| match f {
                        Found::Manual(h) => format!("manual:{}", h.slug),
                        Found::R(h) => format!("r:{}", h.topic),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };
        let got = order(&w);
        assert_eq!(
            got.get(..3),
            Some(
                &[
                    "r:project".to_string(),
                    "manual:projects-and-reporting".to_string(),
                    "r:subproject".to_string()
                ][..]
            ),
            "{got:?}"
        );
        // a page that only mentions the word comes before R's page that only matched it
        let mentions = got.iter().position(|s| s == "r:something");
        let manual_mention = got
            .iter()
            .position(|s| s.starts_with("manual:") && s != "manual:projects-and-reporting");
        assert!(
            manual_mention.is_some() && manual_mention < mentions,
            "{got:?}"
        );
        w.apply_answer(&search, true, "@@DONE\n");
        let got = order(&w);
        assert!(
            !got.is_empty() && got.iter().all(|s| s.starts_with("manual:")),
            "{got:?}"
        );
        assert_eq!(
            got.first().map(String::as_str),
            Some("manual:projects-and-reporting")
        );

        // with no R, a search answers from the manual alone and waits for nothing
        let pal = Palette::new(crate::look::Accent::default(), false);
        let ctx = egui::Context::default();
        let mut w = HelpWindow::default();
        w.request(HelpCommand::Search {
            words: "histogram".into(),
        });
        assert!(w.is_busy());
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = w.show(ctx, &pal, None);
        });
        assert!(!w.is_busy(), "nothing waits for an R that is not there");
        assert!(matches!(
            w.current(),
            Some(Content::Results { found, asked_r: false, .. })
                if matches!(found.first(), Some(Found::Manual(h)) if h.slug == "graphs-dialogs")
        ));
        // ... and a page request still waits for R
        let mut w = HelpWindow::default();
        w.request(page_request("mean"));
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = w.show(ctx, &pal, None);
        });
        assert!(w.is_busy());

        // the manual draws, page by page, without a session, pictures and all
        let mut w = HelpWindow::default();
        let ctx = egui::Context::default();
        for p in manual::PAGES {
            w.show_manual(Some(p.slug));
            assert_eq!(w.shown_manual(), Some(p.slug));
            for _ in 0..3 {
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0));
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = w.show(ctx, &pal, None);
                    },
                );
            }
        }
        // every picture a page shows was made into a texture as it drew
        for pic in manual::PICTURES {
            let id = egui::Id::new(("manual-picture", pic.name));
            assert!(
                ctx.data(|d| d.get_temp::<egui::TextureHandle>(id))
                    .is_some(),
                "{} was drawn",
                pic.name
            );
        }
        // a name that is not a page opens the contents
        w.show_manual(Some("no-such-page"));
        assert_eq!(w.shown_manual(), Some(manual::CONTENTS));
    }

    #[test]
    fn a_request_opens_the_window_and_is_queued() {
        let mut w = HelpWindow::default();
        assert!(!w.open);
        w.request(page_request("mean"));
        assert!(w.open);
        assert_eq!(w.queued, Some(page_request("mean")));
        let mut w = HelpWindow::default();
        w.request(HelpCommand::Search {
            words: "linear".into(),
        });
        assert_eq!(w.query, "linear");
    }

    #[test]
    fn answers_become_pages_results_and_messages() {
        let mut w = HelpWindow::default();
        w.apply_answer(&page_request("mean"), true, MEAN);
        assert!(matches!(w.current(), Some(Content::Page(p)) if p.topic == "mean"));
        w.apply_answer(&page_request("mea"), true, "@@NONE\n@@SUGGEST\tmean\n");
        assert!(
            matches!(w.current(), Some(Content::NotFound { suggestions, .. }) if suggestions == &["mean"])
        );
        let search = HelpCommand::Search { words: "lm".into() };
        w.apply_answer(
            &search,
            true,
            "@@HIT\tstats\tlm\tFitting Linear Models\n@@DONE\n",
        );
        assert!(matches!(w.current(), Some(Content::Results { found, .. })
            if found.iter().filter(|f| matches!(f, Found::R(_))).count() == 1));
        w.apply_answer(&search, true, "half an answer");
        assert!(matches!(w.current(), Some(Content::Message(_))));
        w.apply_answer(&page_request("x"), false, "boom");
        assert!(matches!(w.current(), Some(Content::Message(m)) if m.contains("boom")));
    }

    #[test]
    fn package_vignette_news_and_file_answers_become_content() {
        let mut w = HelpWindow::default();
        let package = HelpCommand::Package {
            package: "demo".into(),
        };
        let page = "@@PACKAGE	demo
@@DESC
|Package: demo
@@END
@@INDEX
|alpha  First
@@END
@@NEWS
@@DONE
";
        w.apply_answer(&package, true, page);
        assert!(
            matches!(w.current(), Some(Content::Package(d)) if d.has_news && d.topics.len() == 1)
        );
        w.apply_answer(
            &package,
            true,
            "@@NONE
@@DONE
",
        );
        assert!(matches!(w.current(), Some(Content::NoPackage(p)) if p == "demo"));
        let vignettes = HelpCommand::Vignettes { package: None };
        w.apply_answer(
            &vignettes,
            true,
            "@@VIG	a	b	Title (source, pdf)
@@VFILE	C:/l/b.pdf
@@DONE
",
        );
        assert!(
            matches!(w.current(), Some(Content::Vignettes { list, .. }) if list.len() == 1 && list[0].files.len() == 1)
        );
        w.apply_answer(&vignettes, true, "half");
        assert!(matches!(w.current(), Some(Content::Message(_))));
        let news = HelpCommand::News {
            package: "demo".into(),
        };
        w.apply_answer(
            &news, true, "@@NONE
",
        );
        assert!(matches!(w.current(), Some(Content::Message(m)) if m.contains("no NEWS")));
        w.apply_answer(
            &news,
            true,
            "@@TITLE	News for demo
@@TEXT
|- fixed
@@END
",
        );
        assert!(matches!(
            w.current(),
            Some(Content::Text {
                runnable: false,
                ..
            })
        ));
        let file = |p: &str| HelpCommand::File { path: p.into() };
        let text = "@@TITLE	b.R
@@TEXT
|x <- 1
@@END
";
        w.apply_answer(&file("C:/l/b.R"), true, text);
        assert!(matches!(
            w.current(),
            Some(Content::Text { runnable: true, .. })
        ));
        w.apply_answer(&file("C:/l/b.Rmd"), true, text);
        assert!(matches!(
            w.current(),
            Some(Content::Text {
                runnable: false,
                ..
            })
        ));
    }

    #[test]
    fn back_and_forward_walk_what_was_shown_and_a_new_page_cuts_the_future() {
        let mut w = HelpWindow::default();
        w.apply_answer(&page_request("mean"), true, MEAN);
        w.apply_answer(&page_request("a"), true, "@@NONE\n");
        w.apply_answer(&page_request("b"), true, "@@NONE\n");
        assert_eq!(w.shown.len(), 3);
        w.at = 0;
        w.apply_answer(&page_request("c"), true, "@@NONE\n");
        assert_eq!(w.shown.len(), 2, "going somewhere new drops the old future");
        assert_eq!(w.at, 1);
    }
}
