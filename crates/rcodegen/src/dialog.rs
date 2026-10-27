//! The dialog-spec format: one declarative [`Spec`] per menu entry gives both
//! the dialog (its widgets, defaults and checks) and the R it runs (its code
//! function, in base R and, where it exists, tidyverse style). The interface
//! draws widgets from the spec and never builds R itself; see
//! `plan/dialog-spec.md`.
//!
//! Everything a code function puts into R comes through [`Args`], which
//! quotes names and strings, so only the text a user types as an R expression
//! reaches R as it is (the same as typing it in the Command line).

use std::collections::BTreeMap;

use rcraft_rdata::Data;

use crate::import::{r_name, r_string};

/// How a column looks to a dialog's variable filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Number,
    Text,
    Factor,
    Logical,
    Date,
    Other,
}

impl VarKind {
    pub fn of(data: &Data) -> VarKind {
        match data {
            Data::Int(_) | Data::Double(_) => VarKind::Number,
            Data::Text(_) => VarKind::Text,
            Data::Factor { .. } => VarKind::Factor,
            Data::Logical(_) => VarKind::Logical,
            Data::Date(_) | Data::DateTime { .. } | Data::Duration { .. } => VarKind::Date,
            Data::Other(_) => VarKind::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VarKind::Number => "number",
            VarKind::Text => "text",
            VarKind::Factor => "factor",
            VarKind::Logical => "logical",
            VarKind::Date => "date",
            VarKind::Other => "other",
        }
    }
}

/// One column of the data frame a dialog works on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnInfo {
    pub name: String,
    pub kind: VarKind,
}

/// What a dialog knows about its data: the data frame's name and columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub dataset: String,
    pub columns: Vec<ColumnInfo>,
}

impl Context {
    pub fn column(&self, name: &str) -> Option<&ColumnInfo> {
        self.columns.iter().find(|c| c.name == name)
    }
}

/// The text a text box starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDefault {
    Empty,
    Fixed(&'static str),
    /// The name of the data frame the dialog works on.
    DatasetName,
}

/// What a text box accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRule {
    /// Anything, including nothing.
    Any,
    /// A name for a variable or data frame; quoted with backticks when R
    /// needs them, so any text without line breaks will do.
    Name,
    /// An R expression, inserted as it is; must not be empty, and brackets
    /// and quotes must balance so it cannot spill out of the call around it.
    Expression,
    /// One finite number, inserted as typed (so `1e3` and `.5` pass, `inf` and
    /// words do not).
    Number,
}

/// One option of a [`Widget::Choice`]: the value code functions see, and the
/// words the dialog shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opt {
    pub value: &'static str,
    pub label: &'static str,
}

/// One control in a dialog. `id` names its value for the code function;
/// `hint` is shown under it (empty for none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    /// One column of the data frame, of the given kinds (empty: any kind).
    Variable {
        id: &'static str,
        label: &'static str,
        kinds: &'static [VarKind],
        required: bool,
        hint: &'static str,
    },
    /// Several columns, in the order chosen: at least `min` of them. With
    /// `min` 0, choosing none means every column of the allowed kinds.
    Variables {
        id: &'static str,
        label: &'static str,
        kinds: &'static [VarKind],
        min: usize,
        hint: &'static str,
    },
    Text {
        id: &'static str,
        label: &'static str,
        default: TextDefault,
        rule: TextRule,
        hint: &'static str,
    },
    /// One of a fixed list; `default` is an option's value.
    Choice {
        id: &'static str,
        label: &'static str,
        options: &'static [Opt],
        default: &'static str,
        hint: &'static str,
    },
    Check {
        id: &'static str,
        label: &'static str,
        default: bool,
        hint: &'static str,
    },
}

impl Widget {
    pub fn id(&self) -> &'static str {
        match self {
            Widget::Variable { id, .. }
            | Widget::Variables { id, .. }
            | Widget::Text { id, .. }
            | Widget::Choice { id, .. }
            | Widget::Check { id, .. } => id,
        }
    }
}

/// Which R a dialog writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// Base R only: works in any R with no package.
    #[default]
    Base,
    /// The tidyverse way, calling `dplyr::` functions by their full names.
    Tidyverse,
}

impl Style {
    pub fn label(self) -> &'static str {
        match self {
            Style::Base => "Base R",
            Style::Tidyverse => "Tidyverse (dplyr)",
        }
    }

    /// Packages the code needs, beyond base R.
    pub fn packages(self) -> &'static [&'static str] {
        match self {
            Style::Base => &[],
            Style::Tidyverse => &["dplyr"],
        }
    }
}

/// Every widget's value, by id: text for variables, text boxes and choices,
/// a flag for check boxes, a list of names for multi-variable widgets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Values {
    texts: BTreeMap<String, String>,
    flags: BTreeMap<String, bool>,
    lists: BTreeMap<String, Vec<String>>,
}

impl Values {
    pub fn set_list(&mut self, id: &str, names: Vec<String>) {
        self.lists.insert(id.to_string(), names);
    }

    /// The names chosen for `id`, in order; empty when unset.
    pub fn list(&self, id: &str) -> &[String] {
        self.lists.get(id).map_or(&[], Vec::as_slice)
    }

    /// The names for `id`, for the interface to edit in place.
    pub fn list_mut(&mut self, id: &str) -> &mut Vec<String> {
        self.lists.entry(id.to_string()).or_default()
    }

    pub fn set_text(&mut self, id: &str, text: impl Into<String>) {
        self.texts.insert(id.to_string(), text.into());
    }

    pub fn set_flag(&mut self, id: &str, on: bool) {
        self.flags.insert(id.to_string(), on);
    }

    /// The text for `id`; empty when unset.
    pub fn text(&self, id: &str) -> &str {
        self.texts.get(id).map_or("", String::as_str)
    }

    pub fn flag(&self, id: &str) -> bool {
        self.flags.get(id).copied().unwrap_or(false)
    }

    /// The text for `id`, for the interface to edit in place.
    pub fn text_mut(&mut self, id: &str) -> &mut String {
        self.texts.entry(id.to_string()).or_default()
    }

    /// The flag for `id`, for the interface to edit in place.
    pub fn flag_mut(&mut self, id: &str) -> &mut bool {
        self.flags.entry(id.to_string()).or_default()
    }
}

/// Something that stops a dialog's code from being made, tied to a widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub widget: &'static str,
    pub message: String,
}

impl Problem {
    pub fn new(widget: &'static str, message: impl Into<String>) -> Problem {
        Problem {
            widget,
            message: message.into(),
        }
    }
}

/// A menu entry's dialog.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The catalog entry's name (`"Sort rows"`).
    pub entry: &'static str,
    /// Where it sits, as the history and log show it (`"Data > Sort rows"`).
    pub menu: &'static str,
    /// One line under the title saying what the dialog does.
    pub about: &'static str,
    pub widgets: &'static [Widget],
    /// The styles it can write, the first being the default.
    pub styles: &'static [Style],
    /// Checks beyond each widget's own (a new name that is taken, say).
    pub check: fn(&Args) -> Vec<Problem>,
    /// The R, once every check has passed.
    pub code: fn(&Args, Style) -> String,
}

/// No checks beyond the widgets' own.
pub fn no_check(_: &Args) -> Vec<Problem> {
    Vec::new()
}

impl Spec {
    /// The values a dialog opens with.
    pub fn defaults(&self, ctx: &Context) -> Values {
        let mut values = Values::default();
        for w in self.widgets {
            match *w {
                Widget::Variable { id, .. } => values.set_text(id, ""),
                Widget::Text { id, default, .. } => values.set_text(
                    id,
                    match default {
                        TextDefault::Empty => String::new(),
                        TextDefault::Fixed(s) => s.to_string(),
                        TextDefault::DatasetName => ctx.dataset.clone(),
                    },
                ),
                Widget::Choice { id, default, .. } => values.set_text(id, default),
                Widget::Check { id, default, .. } => values.set_flag(id, default),
                Widget::Variables { id, .. } => values.set_list(id, Vec::new()),
            }
        }
        values
    }

    /// The columns a variable widget offers.
    pub fn choices<'a>(&self, widget: &Widget, ctx: &'a Context) -> Vec<&'a ColumnInfo> {
        let kinds: &[VarKind] = match widget {
            Widget::Variable { kinds, .. } | Widget::Variables { kinds, .. } => kinds,
            _ => return Vec::new(),
        };
        ctx.columns
            .iter()
            .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
            .collect()
    }

    /// Everything that stops the code from being made; empty when it can be.
    pub fn problems(&self, ctx: &Context, values: &Values) -> Vec<Problem> {
        let mut out = Vec::new();
        for w in self.widgets {
            if let Some(p) = widget_problem(w, ctx, values) {
                out.push(p);
            }
        }
        if out.is_empty() {
            out.extend((self.check)(&Args { ctx, values }));
        }
        out
    }

    /// The R, or why it cannot be made yet.
    pub fn generate(
        &self,
        ctx: &Context,
        values: &Values,
        style: Style,
    ) -> Result<String, Vec<Problem>> {
        let problems = self.problems(ctx, values);
        if !problems.is_empty() {
            return Err(problems);
        }
        let style = if self.styles.contains(&style) {
            style
        } else {
            self.styles.first().copied().unwrap_or_default()
        };
        Ok((self.code)(&Args { ctx, values }, style))
    }
}

fn widget_problem(w: &Widget, ctx: &Context, values: &Values) -> Option<Problem> {
    match *w {
        Widget::Variable {
            id,
            label,
            kinds,
            required,
            ..
        } => {
            let name = values.text(id);
            if name.is_empty() {
                return required.then(|| Problem::new(id, format!("Choose {}.", lower(label))));
            }
            match ctx.column(name) {
                None => Some(Problem::new(
                    id,
                    format!("{name} is not in {}.", ctx.dataset),
                )),
                Some(c) if !kinds.is_empty() && !kinds.contains(&c.kind) => Some(Problem::new(
                    id,
                    format!(
                        "{name} is {}; this needs {}.",
                        c.kind.label(),
                        kinds_text(kinds)
                    ),
                )),
                Some(_) => None,
            }
        }
        Widget::Variables {
            id,
            label,
            kinds,
            min,
            ..
        } => {
            let names = values.list(id);
            if names.len() < min {
                return Some(Problem::new(
                    id,
                    format!(
                        "Choose at least {min} variable{} for {}.",
                        if min == 1 { "" } else { "s" },
                        lower(label)
                    ),
                ));
            }
            names.iter().find_map(|name| match ctx.column(name) {
                None => Some(Problem::new(
                    id,
                    format!("{name} is not in {}.", ctx.dataset),
                )),
                Some(c) if !kinds.is_empty() && !kinds.contains(&c.kind) => Some(Problem::new(
                    id,
                    format!(
                        "{name} is {}; this needs {}.",
                        c.kind.label(),
                        kinds_text(kinds)
                    ),
                )),
                Some(_) => None,
            })
        }
        Widget::Text {
            id, label, rule, ..
        } => {
            let text = values.text(id);
            match rule {
                TextRule::Any => None,
                TextRule::Name => {
                    let name = text.trim();
                    if name.is_empty() {
                        Some(Problem::new(id, format!("Type {}.", lower(label))))
                    } else if name.contains(['\n', '\r', '\0']) {
                        Some(Problem::new(id, "A name cannot have a line break."))
                    } else {
                        None
                    }
                }
                TextRule::Number => {
                    let t = text.trim();
                    if t.is_empty() {
                        Some(Problem::new(id, format!("Type {}.", lower(label))))
                    } else if !t.parse::<f64>().is_ok_and(f64::is_finite) {
                        Some(Problem::new(id, format!("{t} is not a number.")))
                    } else {
                        None
                    }
                }
                TextRule::Expression => {
                    if text.trim().is_empty() {
                        Some(Problem::new(id, format!("Type {}.", lower(label))))
                    } else {
                        expression_problem(text).map(|m| Problem::new(id, m))
                    }
                }
            }
        }
        Widget::Choice { id, options, .. } => {
            let v = values.text(id);
            (!options.iter().any(|o| o.value == v))
                .then(|| Problem::new(id, "Pick one of the options."))
        }
        Widget::Check { .. } => None,
    }
}

fn lower(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn kinds_text(kinds: &[VarKind]) -> String {
    let names: Vec<&str> = kinds.iter().map(|k| k.label()).collect();
    match names.split_last() {
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => "any kind".to_string(),
    }
}

/// Why an expression typed into a dialog cannot go into R's code as it is:
/// a quote left open, or brackets that do not balance (a closing one first
/// would end the call it sits in). Comments and line breaks are refused too.
/// R's parser still has the last word when the code runs.
pub fn expression_problem(text: &str) -> Option<String> {
    let mut stack: Vec<char> = Vec::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            if c == '\n' || c == '\r' {
                return Some("A line break is not allowed here.".into());
            }
            continue;
        }
        match c {
            '"' | '\'' | '`' => quote = Some(c),
            '(' | '[' | '{' => stack.push(c),
            ')' | ']' | '}' => {
                let open = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if stack.pop() != Some(open) {
                    return Some(format!("The {c} has no matching opening bracket."));
                }
            }
            '#' => return Some("A comment (#) would hide the rest of the line.".into()),
            ';' => return Some("Only one expression is allowed (no ;).".into()),
            '\n' | '\r' => return Some("A line break is not allowed here.".into()),
            _ => {}
        }
    }
    if let Some(q) = quote {
        return Some(format!("A {q} is left open."));
    }
    stack.last().map(|c| format!("The {c} is never closed."))
}

/// What a code function reads: the context and the values, already turned
/// into safe R where they are names or strings.
pub struct Args<'a> {
    pub ctx: &'a Context,
    pub values: &'a Values,
}

impl Args<'_> {
    /// The data frame the dialog works on, as R code.
    pub fn data(&self) -> String {
        r_name(&self.ctx.dataset)
    }

    /// A name typed in a text box, as R code (backticks when needed).
    pub fn name(&self, id: &str) -> String {
        r_name(self.values.text(id).trim())
    }

    /// A name typed in a text box, as an R string.
    pub fn name_string(&self, id: &str) -> String {
        r_string(self.values.text(id).trim())
    }

    /// The raw text of a name box (for comparisons in checks).
    pub fn name_text(&self, id: &str) -> &str {
        self.values.text(id).trim()
    }

    /// The chosen column, as R code; `None` when none is chosen.
    pub fn var(&self, id: &str) -> Option<String> {
        let v = self.values.text(id);
        (!v.is_empty()).then(|| r_name(v))
    }

    /// The chosen column, as an R string (for `[[` and `names()`).
    pub fn var_string(&self, id: &str) -> Option<String> {
        let v = self.values.text(id);
        (!v.is_empty()).then(|| r_string(v))
    }

    /// The raw column name chosen (for comparisons in checks).
    pub fn var_text(&self, id: &str) -> &str {
        self.values.text(id)
    }

    /// The columns chosen in a multi-variable widget, in order.
    pub fn vars(&self, id: &str) -> Vec<&str> {
        self.values.list(id).iter().map(String::as_str).collect()
    }

    /// The columns chosen in a multi-variable widget, or, when none is chosen,
    /// every column of the given kinds.
    pub fn vars_or_all(&self, id: &str, kinds: &[VarKind]) -> Vec<&str> {
        let chosen = self.vars(id);
        if !chosen.is_empty() {
            return chosen;
        }
        self.ctx
            .columns
            .iter()
            .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
            .map(|c| c.name.as_str())
            .collect()
    }

    /// A number typed in a [`TextRule::Number`] box, as typed but trimmed.
    pub fn number(&self, id: &str) -> &str {
        self.values.text(id).trim()
    }

    /// An expression the user typed, exactly as typed but trimmed.
    pub fn expr(&self, id: &str) -> &str {
        self.values.text(id).trim()
    }

    pub fn choice(&self, id: &str) -> &str {
        self.values.text(id)
    }

    pub fn flag(&self, id: &str) -> bool {
        self.values.flag(id)
    }

    /// Whether the result named in `id` replaces the data frame itself.
    pub fn replaces_data(&self, id: &str) -> bool {
        self.name_text(id) == self.ctx.dataset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            dataset: "people".into(),
            columns: vec![
                ColumnInfo {
                    name: "age".into(),
                    kind: VarKind::Number,
                },
                ColumnInfo {
                    name: "sex".into(),
                    kind: VarKind::Factor,
                },
            ],
        }
    }

    const WIDGETS: &[Widget] = &[
        Widget::Variable {
            id: "by",
            label: "Sort by",
            kinds: &[VarKind::Number],
            required: true,
            hint: "",
        },
        Widget::Text {
            id: "result",
            label: "Put the result in",
            default: TextDefault::DatasetName,
            rule: TextRule::Name,
            hint: "",
        },
        Widget::Choice {
            id: "dir",
            label: "Order",
            options: &[
                Opt {
                    value: "up",
                    label: "Smallest first",
                },
                Opt {
                    value: "down",
                    label: "Largest first",
                },
            ],
            default: "up",
            hint: "",
        },
    ];

    fn code(a: &Args, style: Style) -> String {
        let by = a.var("by").unwrap_or_default();
        match style {
            Style::Base => format!("{} <- {}[order({}), ]", a.name("result"), a.data(), by),
            Style::Tidyverse => format!("dplyr::arrange({}, {by})", a.data()),
        }
    }

    const SPEC: Spec = Spec {
        entry: "Test",
        menu: "Data > Test",
        about: "",
        widgets: WIDGETS,
        styles: &[Style::Base],
        check: no_check,
        code,
    };

    #[test]
    fn defaults_fill_every_widget() {
        let v = SPEC.defaults(&ctx());
        assert_eq!(v.text("by"), "");
        assert_eq!(v.text("result"), "people");
        assert_eq!(v.text("dir"), "up");
    }

    #[test]
    fn variable_widgets_check_presence_and_kind() {
        let c = ctx();
        let mut v = SPEC.defaults(&c);
        let p = SPEC.problems(&c, &v);
        assert_eq!(p, vec![Problem::new("by", "Choose sort by.")]);
        v.set_text("by", "sex");
        assert_eq!(
            SPEC.problems(&c, &v)[0].message,
            "sex is factor; this needs number."
        );
        v.set_text("by", "height");
        assert_eq!(SPEC.problems(&c, &v)[0].message, "height is not in people.");
        v.set_text("by", "age");
        assert!(SPEC.problems(&c, &v).is_empty());
        assert_eq!(SPEC.choices(&WIDGETS[0], &c).len(), 1);
    }

    #[test]
    fn generate_uses_a_supported_style_and_quotes_names() {
        let c = ctx();
        let mut v = SPEC.defaults(&c);
        v.set_text("by", "age");
        v.set_text("result", " sorted people ");
        assert_eq!(
            SPEC.generate(&c, &v, Style::Tidyverse),
            Ok("`sorted people` <- people[order(age), ]".to_string()),
            "falls back to the only style it has"
        );
        v.set_text("dir", "sideways");
        assert!(SPEC.generate(&c, &v, Style::Base).is_err());
    }

    #[test]
    fn expressions_must_balance_and_stay_one_expression() {
        assert_eq!(expression_problem("age > 30 & sex == \"m\""), None);
        assert_eq!(expression_problem("f(x[1], 'a)')"), None);
        assert_eq!(expression_problem("`odd name` > 1"), None);
        assert_eq!(expression_problem("\"a\\\"b\" == x"), None);
        assert!(expression_problem("x), system(\"y\"").is_some());
        assert!(expression_problem("(x").is_some());
        assert!(expression_problem("x]").is_some());
        assert!(expression_problem("(x]").is_some());
        assert!(expression_problem("\"open").is_some());
        assert!(expression_problem("x # note").is_some());
        assert!(expression_problem("x; y").is_some());
        assert!(expression_problem("x\ny").is_some());
    }

    #[test]
    fn multi_variable_widgets_check_count_names_and_kinds() {
        const MANY: Widget = Widget::Variables {
            id: "vars",
            label: "Variables",
            kinds: &[VarKind::Number],
            min: 2,
            hint: "",
        };
        const SPEC2: Spec = Spec {
            entry: "Test",
            menu: "Data > Test",
            about: "",
            widgets: &[MANY],
            styles: &[Style::Base],
            check: no_check,
            code: |a, _| a.vars("vars").join("+"),
        };
        let c = ctx();
        let mut v = SPEC2.defaults(&c);
        assert!(v.list("vars").is_empty());
        assert_eq!(
            SPEC2.problems(&c, &v)[0].message,
            "Choose at least 2 variables for variables."
        );
        v.set_list("vars", vec!["age".into()]);
        assert!(SPEC2.problems(&c, &v)[0].message.contains("at least 2"));
        v.set_list("vars", vec!["age".into(), "sex".into()]);
        assert_eq!(
            SPEC2.problems(&c, &v)[0].message,
            "sex is factor; this needs number."
        );
        v.set_list("vars", vec!["age".into(), "height".into()]);
        assert_eq!(
            SPEC2.problems(&c, &v)[0].message,
            "height is not in people."
        );
        v.list_mut("vars").clear();
        v.list_mut("vars")
            .extend(["age".to_string(), "age".to_string()]);
        assert_eq!(
            SPEC2.generate(&c, &v, Style::Base),
            Ok("age+age".to_string())
        );
        // only number columns are offered
        assert_eq!(SPEC2.choices(&MANY, &c).len(), 1);
    }

    #[test]
    fn none_chosen_can_mean_every_column_of_the_kinds() {
        let c = ctx();
        let v = Values::default();
        let a = Args {
            ctx: &c,
            values: &v,
        };
        assert_eq!(a.vars_or_all("vars", &[VarKind::Number]), vec!["age"]);
        assert_eq!(a.vars_or_all("vars", &[]), vec!["age", "sex"]);
        let mut chosen = Values::default();
        chosen.set_list("vars", vec!["sex".into()]);
        let a = Args {
            ctx: &c,
            values: &chosen,
        };
        assert_eq!(a.vars_or_all("vars", &[VarKind::Number]), vec!["sex"]);
    }

    #[test]
    fn names_reject_line_breaks_and_empties() {
        let c = ctx();
        let mut v = SPEC.defaults(&c);
        v.set_text("by", "age");
        v.set_text("result", "  ");
        assert_eq!(SPEC.problems(&c, &v)[0].widget, "result");
        v.set_text("result", "a\nb");
        assert_eq!(
            SPEC.problems(&c, &v)[0].message,
            "A name cannot have a line break."
        );
    }

    #[test]
    fn values_edit_in_place() {
        let mut v = Values::default();
        v.text_mut("x").push_str("hi");
        *v.flag_mut("y") = true;
        assert_eq!(v.text("x"), "hi");
        assert!(v.flag("y"));
        assert!(!v.flag("x"));
        assert_eq!(v.text("missing"), "");
    }

    #[test]
    fn kinds_read_as_a_list() {
        assert_eq!(kinds_text(&[VarKind::Number]), "number");
        assert_eq!(
            kinds_text(&[VarKind::Number, VarKind::Factor, VarKind::Text]),
            "number, factor or text"
        );
        assert_eq!(VarKind::of(&Data::Int(vec![])), VarKind::Number);
    }
}
