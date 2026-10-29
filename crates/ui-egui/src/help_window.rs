//! The Help window (P2A.1 to P2A.3, P2A.7): R's own help pages for the
//! installed packages, a search over them, back and forward, links in "See
//! also", and a button that runs a page's examples in the session.
//!
//! The text comes from R (see `rcodegen::help`): a hidden query asks the
//! session for the page, so nothing is added to Results, the log or the
//! history until the user runs an example. Works offline.

use std::time::Duration;

use eframe::egui::{self, RichText, TextStyle};
use rcraft_engine::{Engine, QueryId};
use rcraft_rcodegen::dialog_help::DialogHelp;
use rcraft_rcodegen::help::{
    page_code, page_of, parse_page, parse_search, search_code, HelpCommand, Hit, Page, PageAnswer,
};
use rcraft_rcodegen::help_docs::{
    extension, file_text_code, file_url, kind_of, news_code, package_code, parse_package,
    parse_text, parse_vignettes, vignettes_code, FileKind, PackageAnswer, PackageDoc, Vignette,
};

use crate::look::Palette;

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
        hits: Vec<Hit>,
    },
    Message(String),
    /// The help page of one of RCraft's own dialogs.
    Dialog(Box<DialogHelp>),
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
}

impl HelpWindow {
    /// Opens the window on a request: a page, or a search.
    pub fn request(&mut self, command: HelpCommand) {
        self.open = true;
        if let HelpCommand::Search { words } = &command {
            self.query = words.clone();
        }
        self.queued = Some(command);
    }

    /// Opens the window on the help page of a dialog. Nothing is asked of R.
    pub fn show_dialog(&mut self, page: DialogHelp) {
        self.open = true;
        self.push(Content::Dialog(Box::new(page)));
    }

    /// Opens the window at the search box.
    pub fn open_search(&mut self) {
        self.open = true;
        self.focus_query = true;
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
                        hits,
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

    /// Sends a queued request to R and takes an answer that has come.
    fn service(&mut self, engine: Option<&mut Engine>) {
        let Some(engine) = engine else { return };
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
        let mut open = self.open;
        egui::Window::new("Help")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([700.0, 560.0])
            .default_pos(egui::pos2(240.0, 60.0))
            .show(ctx, |ui| {
                self.toolbar(ui, &mut next);
                ui.separator();
                if self.pending.is_some() || self.queued.is_some() {
                    ui.label(RichText::new("Asking R…").color(pal.dim));
                }
                self.body(ui, pal, ctx, &mut next, &mut out);
            });
        self.open = open;
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

    fn body(
        &self,
        ui: &mut egui::Ui,
        pal: &Palette,
        ctx: &egui::Context,
        next: &mut Option<HelpCommand>,
        out: &mut Option<HelpOut>,
    ) {
        match self.current() {
            None => {
                ui.add_space(8.0);
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
            Some(Content::Results { words, hits }) => {
                ui.label(
                    RichText::new(format!("{} help pages for \"{words}\"", hits.len())).strong(),
                );
                if hits.is_empty() {
                    ui.label("Nothing found in the installed packages.");
                    if ui.button("Find a package for this…").clicked() {
                        *out = Some(HelpOut::FindPackages(words.clone()));
                    }
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for h in hits {
                        ui.horizontal_wrapped(|ui| {
                            if ui.link(RichText::new(&h.topic).strong()).clicked() {
                                *next = Some(HelpCommand::Page {
                                    topic: h.topic.clone(),
                                    package: Some(h.package.clone()),
                                });
                            }
                            ui.label(RichText::new(format!("({})", h.package)).color(pal.dim));
                            ui.label(&h.title);
                        });
                    }
                });
            }
            Some(Content::Page(page)) => self.page(ui, pal, ctx, page, next, out),
            Some(Content::Dialog(page)) => self.dialog_page(ui, pal, ctx, page, next),
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
        assert!(matches!(w.current(), Some(Content::Results { hits, .. }) if hits.len() == 1));
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
