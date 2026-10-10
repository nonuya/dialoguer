use crate::ui::{graph::{Link, Node}, timeline::Timeline};

pub enum Command {
  AddNodeConversation(Node, Timeline),
  AddNodeChoicer(Node),
  RemoveNode {
    index: usize,
    node: Node,
    flow_links: Vec<Link>,
    links: Vec<Link>,
    timeline: Option<Timeline>,
  },
  AddLink {
    link: Link,
    replaced: Option<Link>,
  },
  AddFlowLink {
    link: Link,
    replaced: Option<Link>,
  },
  RemoveLink {
    index: usize,
    link: Link,
  },
  RemoveFlowLink {
    index: usize,
    link: Link,
  },
}
