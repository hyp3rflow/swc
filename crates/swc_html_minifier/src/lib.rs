#![deny(clippy::all)]

use once_cell::sync::Lazy;
use serde_json::Value;
use swc_atoms::{js_word, JsWord};
use swc_cached::regex::CachedRegex;
use swc_common::{collections::AHashSet, sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_css_codegen::{
    writer::basic::{BasicCssWriter, BasicCssWriterConfig},
    CodeGenerator as CssCodeGenerator, CodegenConfig as CssCodegenConfig,
};
use swc_css_parser::parse_file;
use swc_html_ast::*;
use swc_html_codegen::{
    writer::basic::{BasicHtmlWriter, BasicHtmlWriterConfig},
    CodeGenerator as HtmlCodeGenerator, CodegenConfig as HtmlCodegenConfig,
};
use swc_html_parser::parse_file_as_document_fragment;
use swc_html_visit::{VisitMut, VisitMutWith};

use crate::option::{CollapseWhitespaces, MinifyOptions};
pub mod option;

static HTML_BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "controls",
    "default",
    "defer",
    "disabled",
    "formnovalidate",
    "hidden",
    "inert",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "novalidate",
    "open",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "selected",
    // Legacy
    "declare",
    "defaultchecked",
    "defaultmuted",
    "defaultselected",
    "enabled",
    "compact",
    "indeterminate",
    "sortable",
    "nohref",
    "noresize",
    "noshade",
    "truespeed",
    "typemustmatch",
    "nowrap",
    "visible",
    "pauseonexit",
    "scoped",
    "seamless",
];

// Global attributes
static EVENT_HANDLER_ATTRIBUTES: &[&str] = &[
    "onabort",
    "onautocomplete",
    "onautocompleteerror",
    "onauxclick",
    "onbeforematch",
    "oncancel",
    "oncanplay",
    "oncanplaythrough",
    "onchange",
    "onclick",
    "onclose",
    "oncontextlost",
    "oncontextmenu",
    "oncontextrestored",
    "oncuechange",
    "ondblclick",
    "ondrag",
    "ondragend",
    "ondragenter",
    "ondragleave",
    "ondragover",
    "ondragstart",
    "ondrop",
    "ondurationchange",
    "onemptied",
    "onended",
    "onformdata",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    "onmousewheel",
    "onmousedown",
    "onmouseenter",
    "onmouseleave",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
    "onpause",
    "onplay",
    "onplaying",
    "onprogress",
    "onratechange",
    "onreset",
    "onsecuritypolicyviolation",
    "onseeked",
    "onseeking",
    "onselect",
    "onslotchange",
    "onstalled",
    "onsubmit",
    "onsuspend",
    "ontimeupdate",
    "ontoggle",
    "onvolumechange",
    "onwaiting",
    "onwebkitanimationend",
    "onwebkitanimationiteration",
    "onwebkitanimationstart",
    "onwebkittransitionend",
    "onwheel",
    "onblur",
    "onerror",
    "onfocus",
    "onload",
    "onloadeddata",
    "onloadedmetadata",
    "onloadstart",
    "onresize",
    "onscroll",
    "onafterprint",
    "onbeforeprint",
    "onbeforeunload",
    "onhashchange",
    "onlanguagechange",
    "onmessage",
    "onmessageerror",
    "onoffline",
    "ononline",
    "onpagehide",
    "onpageshow",
    "onpopstate",
    "onrejectionhandled",
    "onstorage",
    "onunhandledrejection",
    "onunload",
    "oncut",
    "oncopy",
    "onpaste",
    "onreadystatechange",
    "onvisibilitychange",
    "onshow",
    "onsort",
];

static ALLOW_TO_TRIM_GLOBAL_ATTRIBUTES: &[&str] = &["style", "tabindex", "itemid"];

static ALLOW_TO_TRIM_HTML_ATTRIBUTES: &[(&str, &str)] = &[
    ("head", "profile"),
    ("audio", "src"),
    ("embed", "src"),
    ("iframe", "src"),
    ("img", "src"),
    ("input", "src"),
    ("input", "usemap"),
    ("input", "longdesc"),
    ("script", "src"),
    ("source", "src"),
    ("track", "src"),
    ("video", "src"),
    ("video", "poster"),
    ("td", "colspan"),
    ("td", "rowspan"),
    ("th", "colspan"),
    ("th", "rowspan"),
    ("col", "span"),
    ("colgroup", "span"),
    ("textarea", "cols"),
    ("textarea", "rows"),
    ("textarea", "maxlength"),
    ("input", "size"),
    ("input", "formaction"),
    ("input", "maxlength"),
    ("button", "formaction"),
    ("select", "size"),
    ("form", "action"),
    ("object", "data"),
    ("object", "codebase"),
    ("object", "classid"),
    ("applet", "codebase"),
    ("a", "href"),
    ("area", "href"),
    ("link", "href"),
    ("base", "href"),
    ("q", "cite"),
    ("blockquote", "cite"),
    ("del", "cite"),
    ("ins", "cite"),
    ("img", "usemap"),
    ("object", "usemap"),
];

static COMMA_SEPARATED_GLOBAL_ATTRIBUTES: &[&str] = &["class"];

static COMMA_SEPARATED_HTML_ATTRIBUTES: &[(&str, &str)] = &[
    ("img", "srcset"),
    ("source", "srcset"),
    ("img", "sizes"),
    ("source", "sizes"),
    ("link", "media"),
    ("source", "media"),
    ("style", "media"),
];

static SPACE_SEPARATED_GLOBAL_ATTRIBUTES: &[&str] = &[
    "class",
    "itemtype",
    "itemref",
    "itemprop",
    "accesskey",
    "aria-describedby",
    "aria-labelledby",
    "aria-owns",
];

static SPACE_SEPARATED_HTML_ATTRIBUTES: &[(&str, &str)] = &[
    ("a", "rel"),
    ("a", "ping"),
    ("area", "rel"),
    ("area", "ping"),
    ("link", "rel"),
    ("link", "sizes"),
    ("input", "autocomplete"),
    ("form", "autocomplete"),
    ("iframe", "sandbox"),
    ("td", "headers"),
    ("th", "headers"),
    ("output", "for"),
    ("link", "blocking"),
    ("script", "blocking"),
    ("style", "blocking"),
];

enum MetaElementContentType {
    CommaSeparated,
    SemiSeparated,
}

enum TextChildrenType {
    Json,
    Css,
}

#[inline(always)]
fn is_whitespace(c: char) -> bool {
    matches!(c, '\x09' | '\x0a' | '\x0c' | '\x0d' | '\x20')
}

#[derive(Debug, Copy, Clone)]
struct WhitespaceMinificationMode {
    pub trim: bool,
    pub destroy_whole: bool,
    pub collapse: bool,
}

pub static CONDITIONAL_COMMENT_START: Lazy<CachedRegex> =
    Lazy::new(|| CachedRegex::new("^\\[if\\s[^\\]+]").unwrap());

pub static CONDITIONAL_COMMENT_END: Lazy<CachedRegex> =
    Lazy::new(|| CachedRegex::new("\\[endif]").unwrap());

struct Minifier {
    current_element_namespace: Option<Namespace>,
    current_element_tag_name: Option<JsWord>,

    current_element_text_children_type: Option<TextChildrenType>,

    meta_element_content_type: Option<MetaElementContentType>,

    force_set_html5_doctype: bool,

    descendant_of_pre: bool,
    collapse_whitespaces: Option<CollapseWhitespaces>,

    remove_empty_attributes: bool,
    collapse_boolean_attributes: bool,
    minify_css: bool,
    preserve_comments: Option<Vec<CachedRegex>>,
    minify_conditional_comments: bool,
}

impl Minifier {
    fn is_event_handler_attribute(&self, name: &str) -> bool {
        EVENT_HANDLER_ATTRIBUTES.contains(&name)
    }

    fn is_boolean_attribute(&self, name: &str) -> bool {
        HTML_BOOLEAN_ATTRIBUTES.contains(&name)
    }

    fn is_global_trimable_attribute(&self, name: &str) -> bool {
        ALLOW_TO_TRIM_GLOBAL_ATTRIBUTES.contains(&name)
    }

    fn is_html_trimable_attribute(
        &self,
        tag_name: Option<&JsWord>,
        attribute_name: &JsWord,
    ) -> bool {
        let tag_name = match tag_name {
            Some(tag_name) => tag_name,
            _ => return false,
        };

        ALLOW_TO_TRIM_HTML_ATTRIBUTES.contains(&(tag_name, attribute_name))
    }

    fn is_global_comma_separated_attribute(&self, name: &str) -> bool {
        COMMA_SEPARATED_GLOBAL_ATTRIBUTES.contains(&name)
    }

    fn is_html_comma_separated_attribute(
        &self,
        tag_name: Option<&JsWord>,
        attribute_name: &JsWord,
    ) -> bool {
        let tag_name = match tag_name {
            Some(tag_name) => tag_name,
            _ => return false,
        };

        COMMA_SEPARATED_HTML_ATTRIBUTES.contains(&(tag_name, attribute_name))
    }

    fn is_global_space_separated_attribute(&self, name: &str) -> bool {
        SPACE_SEPARATED_GLOBAL_ATTRIBUTES.contains(&name)
    }

    fn is_html_space_separated_attribute(
        &self,
        tag_name: Option<&JsWord>,
        attribute_name: &JsWord,
    ) -> bool {
        let tag_name = match tag_name {
            Some(tag_name) => tag_name,
            _ => return false,
        };

        SPACE_SEPARATED_HTML_ATTRIBUTES.contains(&(tag_name, attribute_name))
    }

    fn is_default_attribute_value(
        &self,
        namespace: Namespace,
        tag_name: &str,
        attribute_name: &str,
        attribute_value: &str,
    ) -> bool {
        matches!(
            (
                namespace,
                tag_name,
                attribute_name,
                attribute_value.to_ascii_lowercase().trim()
            ),
            (Namespace::HTML, "iframe", "height", "150")
                | (Namespace::HTML, "iframe", "width", "300")
                | (Namespace::HTML, "iframe", "frameborder", "1")
                | (Namespace::HTML, "iframe", "loading", "eager")
                | (Namespace::HTML, "iframe", "fetchpriority", "auto")
                | (
                    Namespace::HTML,
                    "iframe",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (
                    Namespace::HTML,
                    "a",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (Namespace::HTML, "a", "target", "_self")
                | (Namespace::HTML, "area", "target", "_self")
                | (Namespace::HTML, "area", "shape", "rect")
                | (
                    Namespace::HTML,
                    "area",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (Namespace::HTML, "form", "method", "get")
                | (Namespace::HTML, "form", "target", "_self")
                | (
                    Namespace::HTML,
                    "form",
                    "enctype",
                    "application/x-www-form-urlencoded"
                )
                | (Namespace::HTML, "input", "type", "text")
                | (Namespace::HTML, "input", "size", "20")
                | (Namespace::HTML, "track", "kind", "subtitles")
                | (Namespace::HTML, "textarea", "cols", "20")
                | (Namespace::HTML, "textarea", "rows", "2")
                | (Namespace::HTML, "textarea", "wrap", "sort")
                | (Namespace::HTML, "progress", "max", "1")
                | (Namespace::HTML, "meter", "min", "0")
                | (Namespace::HTML, "img", "decoding", "auto")
                | (Namespace::HTML, "img", "fetchpriority", "auto")
                | (Namespace::HTML, "img", "loading", "eager")
                | (
                    Namespace::HTML,
                    "img",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (Namespace::HTML, "link", "type", "text/css")
                | (Namespace::HTML, "link", "fetchpriority", "auto")
                | (
                    Namespace::HTML,
                    "link",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (Namespace::HTML, "style", "type", "text/css")
                | (Namespace::HTML, "script", "language", "javascript")
                | (Namespace::HTML, "script", "language", "javascript1.2")
                | (Namespace::HTML, "script", "language", "javascript1.3")
                | (Namespace::HTML, "script", "language", "javascript1.4")
                | (Namespace::HTML, "script", "language", "javascript1.5")
                | (Namespace::HTML, "script", "language", "javascript1.6")
                | (Namespace::HTML, "script", "language", "javascript1.7")
                | (Namespace::HTML, "script", "type", "text/javascript")
                | (Namespace::HTML, "script", "type", "text/ecmascript")
                | (Namespace::HTML, "script", "type", "text/jscript")
                | (Namespace::HTML, "script", "type", "application/javascript")
                | (
                    Namespace::HTML,
                    "script",
                    "type",
                    "application/x-javascript"
                )
                | (Namespace::HTML, "script", "type", "application/ecmascript")
                | (Namespace::HTML, "script", "fetchpriority", "auto")
                | (
                    Namespace::HTML,
                    "script",
                    "referrerpolicy",
                    "strict-origin-when-cross-origin"
                )
                | (Namespace::HTML, "ol", "type", "1")
                | (Namespace::HTML, "base", "target", "_self")
                | (Namespace::HTML, "canvas", "height", "150")
                | (Namespace::HTML, "canvas", "width", "300")
                | (Namespace::HTML, "col", "span", "1")
                | (Namespace::HTML, "colgroup", "span", "1")
                | (Namespace::HTML, "td", "colspan", "1")
                | (Namespace::HTML, "td", "rowspan", "1")
                | (Namespace::HTML, "th", "colspan", "1")
                | (Namespace::HTML, "th", "rowspan", "1")
                | (Namespace::HTML, "button", "type", "submit")
        )
    }

    fn is_preserved_comment(&self, data: &str) -> bool {
        if let Some(preserve_comments) = &self.preserve_comments {
            return preserve_comments.iter().any(|regex| regex.is_match(data));
        }

        false
    }

    fn is_conditional_comment(&self, data: &str) -> bool {
        if CONDITIONAL_COMMENT_START.is_match(data) || CONDITIONAL_COMMENT_END.is_match(data) {
            return true;
        }

        false
    }

    fn get_whitespace_minification_for_tag(
        &self,
        mode: &CollapseWhitespaces,
        namespace: Namespace,
        tag_name: &str,
    ) -> WhitespaceMinificationMode {
        let default_destroy_whole = match mode {
            CollapseWhitespaces::All => true,
            CollapseWhitespaces::Smart | CollapseWhitespaces::Conservative => false,
        };
        let default_trim = match mode {
            CollapseWhitespaces::All => true,
            CollapseWhitespaces::Smart | CollapseWhitespaces::Conservative => false,
        };

        match namespace {
            Namespace::HTML => {
                match tag_name {
                    // Inline text semantics + legacy tags + `del` + `ins` - `br`
                    "a" | "abbr" | "acronym" | "b" | "bdi" | "bdo" | "cite" | "data" | "big"
                    | "del" | "dfn" | "em" | "i" | "ins" | "kbd" | "mark" | "q" | "nobr" | "rp"
                    | "rt" | "rtc" | "ruby" | "s" | "samp" | "small" | "span" | "strike"
                    | "strong" | "sub" | "sup" | "time" | "tt" | "u" | "var" | "wbr" => {
                        WhitespaceMinificationMode {
                            collapse: true,
                            destroy_whole: default_destroy_whole,
                            trim: default_trim,
                        }
                    }
                    "script" | "style" => WhitespaceMinificationMode {
                        collapse: false,
                        destroy_whole: true,
                        trim: true,
                    },
                    "textarea" | "code" | "pre" | "listing" | "plaintext" | "xmp" => {
                        WhitespaceMinificationMode {
                            collapse: false,
                            destroy_whole: false,
                            trim: false,
                        }
                    }
                    _ => WhitespaceMinificationMode {
                        collapse: true,
                        destroy_whole: default_destroy_whole,
                        trim: default_trim,
                    },
                }
            }
            Namespace::SVG => match tag_name {
                "desc" | "text" | "title" => WhitespaceMinificationMode {
                    collapse: true,
                    destroy_whole: true,
                    trim: true,
                },
                "a" | "altGlyph" | "tspan" | "textPath" | "tref" => WhitespaceMinificationMode {
                    collapse: true,
                    destroy_whole: default_destroy_whole,
                    trim: default_trim,
                },
                _ => WhitespaceMinificationMode {
                    collapse: true,
                    destroy_whole: true,
                    trim: true,
                },
            },
            _ => WhitespaceMinificationMode {
                collapse: false,
                destroy_whole: false,
                trim: false,
            },
        }
    }

    fn collapse_whitespace(&self, data: &str) -> String {
        let mut collapsed = String::with_capacity(data.len());
        let mut in_whitespace = false;

        for c in data.chars() {
            if is_whitespace(c) {
                if in_whitespace {
                    // Skip this character.
                    continue;
                };

                in_whitespace = true;

                collapsed.push(' ');
            } else {
                in_whitespace = false;

                collapsed.push(c);
            };
        }

        collapsed
    }

    // TODO source map url output?
    fn minify_css(&self, data: String) -> Option<String> {
        let mut errors: Vec<_> = vec![];

        let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
        let fm = cm.new_source_file(FileName::Anon, data);

        let mut stylesheet = match parse_file(&fm, Default::default(), &mut errors) {
            Ok(stylesheet) => stylesheet,
            _ => return None,
        };

        // Avoid compress potential invalid CSS
        if !errors.is_empty() {
            return None;
        }

        swc_css_minifier::minify(&mut stylesheet);

        let mut minified = String::new();
        let wr = BasicCssWriter::new(&mut minified, None, BasicCssWriterConfig::default());
        let mut gen = CssCodeGenerator::new(wr, CssCodegenConfig { minify: true });

        swc_css_codegen::Emit::emit(&mut gen, &stylesheet).unwrap();

        Some(minified)
    }

    fn minify_html(&self, data: String) -> Option<String> {
        let mut errors: Vec<_> = vec![];

        let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
        let fm = cm.new_source_file(FileName::Anon, data);

        // Emulate content inside conditional comments like content inside the
        // `template` element
        let context_element = Element {
            span: Default::default(),
            tag_name: "template".into(),
            namespace: Namespace::HTML,
            attributes: vec![],
            children: vec![],
            content: None,
            is_self_closing: false,
        };
        let mut document_fragment = match parse_file_as_document_fragment(
            &fm,
            context_element.clone(),
            Default::default(),
            &mut errors,
        ) {
            Ok(document_fragment) => document_fragment,
            _ => return None,
        };

        // Avoid compress potential invalid CSS
        if !errors.is_empty() {
            return None;
        }

        document_fragment.visit_mut_with(&mut Minifier {
            current_element_namespace: None,
            current_element_tag_name: None,

            current_element_text_children_type: None,

            meta_element_content_type: None,

            force_set_html5_doctype: self.force_set_html5_doctype,

            descendant_of_pre: false,
            collapse_whitespaces: self.collapse_whitespaces.clone(),

            remove_empty_attributes: self.remove_empty_attributes,
            collapse_boolean_attributes: self.collapse_boolean_attributes,

            minify_css: self.minify_css,

            preserve_comments: self.preserve_comments.clone(),

            minify_conditional_comments: self.minify_conditional_comments,
        });

        let mut minified = String::new();
        let wr = BasicHtmlWriter::new(&mut minified, None, BasicHtmlWriterConfig::default());
        let mut gen = HtmlCodeGenerator::new(
            wr,
            HtmlCodegenConfig {
                minify: true,
                scripting_enabled: false,
                context_element: Some(context_element),
                tag_omission: None,
                self_closing_void_elements: None,
            },
        );

        swc_html_codegen::Emit::emit(&mut gen, &document_fragment).unwrap();

        Some(minified)
    }
}

impl VisitMut for Minifier {
    fn visit_mut_document(&mut self, n: &mut Document) {
        n.visit_mut_children_with(self);

        n.children
            .retain(|child| !matches!(child, Child::Comment(_)));
    }

    fn visit_mut_document_fragment(&mut self, n: &mut DocumentFragment) {
        n.visit_mut_children_with(self);

        n.children
            .retain(|child| !matches!(child, Child::Comment(_)));
    }

    fn visit_mut_document_type(&mut self, n: &mut DocumentType) {
        n.visit_mut_children_with(self);

        if !self.force_set_html5_doctype {
            return;
        }

        n.name = Some("html".into());
        n.system_id = None;
        n.public_id = None;
    }

    fn visit_mut_element(&mut self, n: &mut Element) {
        self.current_element_namespace = Some(n.namespace);
        self.current_element_tag_name = Some(n.tag_name.clone());

        let old_descendant_of_pre = self.descendant_of_pre;
        let whitespace_minification_mode = match &self.collapse_whitespaces {
            Some(mode) => {
                Some(self.get_whitespace_minification_for_tag(mode, n.namespace, &n.tag_name))
            }
            _ => None,
        };

        if n.namespace == Namespace::HTML {
            match &*n.tag_name {
                "meta" => {
                    if n.attributes.iter().any(|attribute| {
                        match &*attribute.name.to_ascii_lowercase() {
                            "name"
                                if attribute.value.is_some()
                                    && &*attribute.value.as_ref().unwrap().to_ascii_lowercase()
                                        == "viewport" =>
                            {
                                true
                            }
                            _ => false,
                        }
                    }) {
                        self.meta_element_content_type =
                            Some(MetaElementContentType::CommaSeparated);
                    } else if n.attributes.iter().any(|attribute| {
                        match &*attribute.name.to_ascii_lowercase() {
                            "http-equiv"
                                if attribute.value.is_some()
                                    && &*attribute.value.as_ref().unwrap().to_ascii_lowercase()
                                        == "content-security-policy" =>
                            {
                                true
                            }
                            _ => false,
                        }
                    }) {
                        self.meta_element_content_type =
                            Some(MetaElementContentType::SemiSeparated);
                    } else {
                        self.meta_element_content_type = None;
                    }
                }
                "script"
                    if n.attributes.iter().any(|attribute| match &*attribute.name {
                        "type"
                            if attribute.value.is_some()
                                && matches!(
                                    &**attribute.value.as_ref().unwrap(),
                                    "application/json"
                                        | "application/ld+json"
                                        | "importmap"
                                        | "speculationrules"
                                ) =>
                        {
                            true
                        }
                        _ => false,
                    }) =>
                {
                    self.current_element_text_children_type = Some(TextChildrenType::Json);
                }
                "style" if self.minify_css => {
                    let mut type_attribute_value = None;

                    for attribute in &n.attributes {
                        if &*attribute.name == "type" && attribute.value.is_some() {
                            type_attribute_value = Some(
                                attribute
                                    .value
                                    .as_ref()
                                    .unwrap()
                                    .trim()
                                    .to_ascii_lowercase(),
                            );

                            break;
                        }
                    }

                    if type_attribute_value.is_none()
                        || type_attribute_value == Some("text/css".into())
                    {
                        self.current_element_text_children_type = Some(TextChildrenType::Css);
                    } else {
                        self.current_element_text_children_type = None;
                    }

                    self.meta_element_content_type = None;
                }
                "pre" if whitespace_minification_mode.is_some() => {
                    self.descendant_of_pre = true;
                }
                _ => {
                    self.meta_element_content_type = None;
                    self.current_element_text_children_type = None;
                }
            }
        }

        let mut index = 0;
        let last = n.children.len();

        n.children.retain_mut(|child| {
            index += 1;

            match child {
                Child::Comment(comment) if !self.is_preserved_comment(&comment.data) => false,
                // Always remove whitespaces from html and head elements (except nested elements),
                // it should be safe
                Child::Text(_)
                    if matches!(&*n.tag_name, "html" | "head")
                        && n.namespace == Namespace::HTML =>
                {
                    false
                }
                Child::Text(text)
                    if whitespace_minification_mode.is_some() && !self.descendant_of_pre =>
                {
                    let mode = whitespace_minification_mode.unwrap();

                    let value = if mode.trim
                        && (index == 1
                            || self.collapse_whitespaces == Some(CollapseWhitespaces::All))
                    {
                        text.data.trim_start_matches(is_whitespace)
                    } else {
                        &*text.data
                    };

                    let value = if mode.trim
                        && (index == last
                            || self.collapse_whitespaces == Some(CollapseWhitespaces::All))
                    {
                        value.trim_end_matches(is_whitespace)
                    } else {
                        value
                    };

                    if mode.destroy_whole && value.chars().all(is_whitespace) {
                        false
                    } else if mode.collapse {
                        text.data = self.collapse_whitespace(value).into();

                        true
                    } else if value.is_empty() {
                        false
                    } else {
                        text.data = value.into();

                        true
                    }
                }
                _ => true,
            }
        });

        n.visit_mut_children_with(self);

        if whitespace_minification_mode.is_some() {
            self.descendant_of_pre = old_descendant_of_pre;
        }

        let mut already_seen: AHashSet<JsWord> = Default::default();

        n.attributes.retain(|attribute| {
            if already_seen.contains(&attribute.name) {
                return false;
            }

            already_seen.insert(attribute.name.clone());

            if attribute.value.is_none() {
                return true;
            }

            if self.is_default_attribute_value(
                n.namespace,
                &n.tag_name,
                &attribute.name,
                match &*n.tag_name {
                    "script" if n.namespace == Namespace::HTML => {
                        let original_value = attribute.value.as_ref().unwrap();

                        if let Some(next) = original_value.split(';').next() {
                            next
                        } else {
                            original_value
                        }
                    }
                    _ => attribute.value.as_ref().unwrap(),
                },
            ) {
                return false;
            }

            if self.remove_empty_attributes {
                let value = attribute.value.as_ref().unwrap();

                if (matches!(&*attribute.name, "id") && value.is_empty())
                    || (matches!(&*attribute.name, "class" | "style") && value.is_empty())
                    || self.is_event_handler_attribute(&attribute.name) && value.is_empty()
                {
                    return false;
                }
            }

            true
        });
    }

    fn visit_mut_attribute(&mut self, n: &mut Attribute) {
        n.visit_mut_children_with(self);

        if n.value.is_none() {
            return;
        }

        let mut value = match &n.value {
            Some(value) => value.to_string(),
            _ => {
                unreachable!();
            }
        };

        let is_element_html_namespace = self.current_element_namespace == Some(Namespace::HTML);

        if self.collapse_boolean_attributes
            && is_element_html_namespace
            && self.is_boolean_attribute(&n.name)
        {
            n.value = None;

            return;
        } else if self.is_global_space_separated_attribute(&n.name)
            || (is_element_html_namespace
                && self.is_html_space_separated_attribute(
                    self.current_element_tag_name.as_ref(),
                    &n.name,
                ))
        {
            let mut values = value.split_whitespace().collect::<Vec<_>>();

            if &*n.name == "class" {
                values.sort_unstable();
            }

            value = values.join(" ");
        } else if self.is_global_comma_separated_attribute(&n.name)
            || (is_element_html_namespace
                && ((&n.name == "content"
                    && matches!(
                        self.meta_element_content_type,
                        Some(MetaElementContentType::CommaSeparated)
                    ))
                    || self.is_html_comma_separated_attribute(
                        self.current_element_tag_name.as_ref(),
                        &n.name,
                    )))
        {
            let values = value.trim().split(',');

            let mut new_values = vec![];

            for value in values {
                new_values.push(value.trim());
            }

            value = new_values.join(",");
        } else if self.is_global_trimable_attribute(&n.name)
            || (is_element_html_namespace
                && self.is_html_trimable_attribute(self.current_element_tag_name.as_ref(), &n.name))
        {
            value = value.trim().to_string();
        } else if is_element_html_namespace && &n.name == "contenteditable" && value == "true" {
            n.value = Some(js_word!(""));

            return;
        } else if &n.name == "content"
            && matches!(
                self.meta_element_content_type,
                Some(MetaElementContentType::SemiSeparated)
            )
        {
            let values = value.trim().split(';');

            let mut new_values = vec![];

            for value in values {
                new_values.push(
                    value
                        .trim()
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }

            value = new_values.join(";");

            if value.ends_with(';') {
                value.pop();
            }
        } else if self.is_event_handler_attribute(&n.name) {
            value = value.trim().into();

            if value.trim().to_lowercase().starts_with("javascript:") {
                value = value.chars().skip(11).collect();
            }
        }

        n.value = Some(value.into());
    }

    fn visit_mut_text(&mut self, n: &mut Text) {
        n.visit_mut_children_with(self);

        match self.current_element_text_children_type {
            Some(TextChildrenType::Json) if n.data.len() > 0 => {
                let json = match serde_json::from_str::<Value>(&*n.data) {
                    Ok(json) => json,
                    _ => return,
                };
                let minified = match serde_json::to_string(&json) {
                    Ok(minified_json) => minified_json,
                    _ => return,
                };

                n.data = minified.into()
            }
            Some(TextChildrenType::Css) if n.data.len() > 0 => {
                let minified = match self.minify_css(n.data.to_string()) {
                    Some(minified) => minified,
                    None => return,
                };

                n.data = minified.into()
            }
            _ => {}
        }
    }

    fn visit_mut_comment(&mut self, n: &mut Comment) {
        n.visit_mut_children_with(self);

        if !self.minify_conditional_comments {
            return;
        }

        if self.is_conditional_comment(&n.data) && n.data.len() > 0 {
            let start_pos = match n.data.find("]>") {
                Some(start_pos) => start_pos,
                _ => return,
            };
            let end_pos = match n.data.find("<![") {
                Some(end_pos) => end_pos,
                _ => return,
            };

            let html = n
                .data
                .chars()
                .skip(start_pos)
                .take(end_pos - start_pos)
                .collect();

            let minified = match self.minify_html(html) {
                Some(minified) => minified,
                _ => return,
            };
            let before: String = n.data.chars().take(start_pos).collect();
            let after: String = n.data.chars().skip(end_pos).take(n.data.len()).collect();
            let mut data = String::with_capacity(n.data.len());

            data.push_str(&before);
            data.push_str(&minified);
            data.push_str(&after);

            n.data = data.into();
        }
    }
}

pub fn minify(document: &mut Document, options: &MinifyOptions) {
    document.visit_mut_with(&mut Minifier {
        current_element_namespace: None,
        current_element_tag_name: None,

        current_element_text_children_type: None,

        meta_element_content_type: None,

        force_set_html5_doctype: options.force_set_html5_doctype,

        descendant_of_pre: false,
        collapse_whitespaces: options.collapse_whitespaces.clone(),

        remove_empty_attributes: options.remove_empty_attributes,
        collapse_boolean_attributes: options.collapse_boolean_attributes,

        minify_css: options.minify_css,

        preserve_comments: options.preserve_comments.clone(),
        minify_conditional_comments: options.minify_conditional_comments,
    });
}
