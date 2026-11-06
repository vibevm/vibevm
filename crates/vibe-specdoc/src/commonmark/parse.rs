//! Container-scoped outline from one upstream event stream, without reparsing text.
use super::*;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
struct Frame {
    node: NodeId,
    sections: Vec<(u8, NodeId)>,
}
struct Builder {
    nodes: Vec<Node>,
    frames: Vec<Frame>,
    inline: Vec<NodeId>,
    heading: Option<NodeId>,
}
pub(super) fn parse(source: &str) -> CommonMarkDoc {
    let mut builder = Builder {
        nodes: vec![Node {
            id: NodeId(0),
            kind: NodeKind::Document,
            span: 0..source.len(),
            parent: None,
            children: Vec::new(),
        }],
        frames: vec![Frame {
            node: NodeId(0),
            sections: Vec::new(),
        }],
        inline: Vec::new(),
        heading: None,
    };
    for (event, span) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        builder.event(event, span, source.len());
    }
    while let Some(frame) = builder.frames.pop() {
        builder.close_sections(frame.sections, source.len());
    }
    CommonMarkDoc {
        source: source.to_owned(),
        nodes: builder.nodes,
    }
}
impl Builder {
    fn parent(&self) -> NodeId {
        self.inline
            .last()
            .copied()
            .or(self.heading)
            .or_else(|| {
                self.frames.last().map(|frame| {
                    frame
                        .sections
                        .last()
                        .map(|(_, id)| *id)
                        .unwrap_or(frame.node)
                })
            })
            .unwrap_or(NodeId(0))
    }
    fn push(&mut self, kind: NodeKind, span: Range<usize>) -> NodeId {
        let parent = self.parent();
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            id,
            kind,
            span,
            parent: Some(parent),
            children: Vec::new(),
        });
        if let Some(node) = self.nodes.get_mut(parent.0) {
            node.children.push(id);
        }
        id
    }
    fn end(&mut self, id: NodeId, end: usize) {
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.span.end = end;
        }
    }
    fn close_sections(&mut self, sections: Vec<(u8, NodeId)>, end: usize) {
        for (_, id) in sections {
            self.end(id, end);
        }
    }
    fn container(&mut self, kind: ContainerKind, span: Range<usize>) {
        let id = self.push(NodeKind::Container(kind), span);
        self.frames.push(Frame {
            node: id,
            sections: Vec::new(),
        });
    }
    fn text(&mut self, text: &str, span: Range<usize>, code: bool, visible: bool) {
        if visible {
            if let Some(id) = self.heading
                && let Some(Node {
                    kind: NodeKind::Section { title, .. },
                    ..
                }) = self.nodes.get_mut(id.0)
            {
                title.push_str(text);
            }
            for id in &self.inline {
                if let Some(Node {
                    kind: NodeKind::Link { label, .. } | NodeKind::Image { label, .. },
                    ..
                }) = self.nodes.get_mut(id.0)
                {
                    label.push_str(text);
                }
            }
        }
        self.push(
            if code {
                NodeKind::InlineCode(text.to_owned())
            } else {
                NodeKind::Text(text.to_owned())
            },
            span,
        );
    }
    fn event(&mut self, event: Event<'_>, span: Range<usize>, source_end: usize) {
        match event {
            Event::Start(Tag::BlockQuote(_)) => self.container(ContainerKind::Quote, span),
            Event::Start(Tag::List(_)) => self.container(ContainerKind::List, span),
            Event::Start(Tag::Item) => self.container(ContainerKind::Item, span),
            Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item) => {
                if self.frames.len() > 1
                    && let Some(frame) = self.frames.pop()
                {
                    self.end(frame.node, span.end);
                    self.close_sections(frame.sections, span.end);
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level = level as u8;
                let mut closed = Vec::new();
                if let Some(frame) = self.frames.last_mut() {
                    while frame
                        .sections
                        .last()
                        .is_some_and(|(prior, _)| *prior >= level)
                    {
                        if let Some(row) = frame.sections.pop() {
                            closed.push(row);
                        }
                    }
                }
                self.close_sections(closed, span.start);
                let id = self.push(
                    NodeKind::Section {
                        level,
                        title: String::new(),
                        heading_span: span.clone(),
                    },
                    span.start..source_end,
                );
                if let Some(frame) = self.frames.last_mut() {
                    frame.sections.push((level, id));
                }
                self.heading = Some(id);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(id) = self.heading.take()
                    && let Some(Node {
                        kind:
                            NodeKind::Section {
                                title,
                                heading_span,
                                ..
                            },
                        ..
                    }) = self.nodes.get_mut(id.0)
                {
                    *title = normalized(title);
                    heading_span.end = span.end;
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Indented => String::new(),
                    CodeBlockKind::Fenced(info) => info.into_string(),
                };
                let id = self.push(NodeKind::Fence { info }, span);
                self.inline.push(id);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let id = self.push(
                    NodeKind::Link {
                        destination: dest_url.into_string(),
                        label: String::new(),
                    },
                    span,
                );
                self.inline.push(id);
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let id = self.push(
                    NodeKind::Image {
                        destination: dest_url.into_string(),
                        label: String::new(),
                    },
                    span,
                );
                self.inline.push(id);
            }
            Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image) => {
                if let Some(id) = self.inline.pop() {
                    self.end(id, span.end);
                    if let Some(Node {
                        kind: NodeKind::Link { label, .. } | NodeKind::Image { label, .. },
                        ..
                    }) = self.nodes.get_mut(id.0)
                    {
                        *label = normalized(label);
                    }
                }
            }
            Event::Text(text) => self.text(&text, span, false, true),
            Event::Code(text) => self.text(&text, span, true, true),
            Event::Html(text) | Event::InlineHtml(text) => self.text(&text, span, false, false),
            Event::SoftBreak | Event::HardBreak => self.text(" ", span, false, true),
            _ => {}
        }
    }
}
