fn main() {
    println!("Hello, world!");
}

mod dom {
    use std::collections::{HashMap, HashSet};

    struct Parser {
        pos: usize,
        input: String,
    }

    type PropertyMap = HashMap<String, String>;

    struct StyledNode<'a> {
        node: &'a Node,
        specified_values: PropertyMap,
        children: Vec<StyledNode<'a>>,
    }

    struct Stylesheet {
        rules: Vec<Rule>,
    }

    struct Rule {
        selectors: Vec<Selector>,
        declarations: Vec<Declaration>,
    }

    struct SimpleSelector {
        tag_name: Option<String>,
        id: Option<String>,
        class: Vec<String>,
    }

    struct Declaration {
        name: String,
        value: Value,
    }

    enum Value {
        Keyword(String),
        Length(f32, Unit),
        ColorValue(Color),
    }

    enum Unit {
        Px,
    }

    struct Color {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    }

    pub enum Selector {
        Simple(SimpleSelector),
    }

    pub type Specificity = (usize, usize, usize);

    impl Selector {
        pub fn specificity(&self) -> Specificity {
            let Selector::Simple(ref simple) = *self;
            let a = simple.id.iter().count();
            let b = simple.class.len();
            let c = simple.tag_name.iter().count();

            return (a, b, c);
        }
    }

    fn matches(elem: &ElementData, selector: &Selector) -> bool {
        match *selector {
            Selector::Simple(ref simple_select) => matches_simple_selector(elem, simple_select),
        }
    }

    impl ElementData {
        pub fn id(&self) -> Option<&String> {
            self.attributes.get("id")
        }

        pub fn classes(&self) -> HashSet<&str> {
            match self.attributes.get("class") {
                Some(classlist) => classlist.split(' ').collect(),
                None => HashSet::new(),
            }
        }
    }

    fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
        if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
            return false;
        }

        if selector.id.iter().any(|id| elem.id() != Some(id)) {
            return false;
        }

        let elem_classes = elem.classes();

        if selector
            .class
            .iter()
            .any(|class| !elem_classes.contains(&**class))
        {
            return false;
        }

        return true;
    }

    type MatchedRule<'a> = (Specificity, &'a Rule);

    fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
        rule.selectors
            .iter()
            .find(|selector| (matches(elem, *selector)))
            .map(|selector| (selector.specificity(), rule))
    }

    fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
        stylesheet.rules.iter().filter_map(|rule| {
            match_rule(elem, rule)
        }).collect()
    }

    impl Parser {
        fn next_char(&self) -> char {
            self.input[self.pos..].chars().next().unwrap()
        }

        fn starts_with(&self, s: &str) -> bool {
            self.input[self.pos..].starts_with(s)
        }

        fn eof(&self) -> bool {
            self.pos >= self.input.len()
        }

        fn consume_char(&mut self) -> char {
            let mut iter = self.input[self.pos..].char_indices();
            let (_, cur_char) = iter.next().unwrap();
            let (next_pos, _) = iter.next().unwrap_or((1, ' '));
            self.pos += next_pos;
            return cur_char;
        }

        fn consume_while<T>(&mut self, test: T) -> String
        where
            T: Fn(char) -> bool,
        {
            let mut result = String::new();
            while !self.eof() && test(self.next_char()) {
                result.push(self.consume_char());
            }

            result
        }

        fn consume_whitespace(&mut self) {
            self.consume_while(char::is_whitespace);
        }

        fn parse_tag_name(&mut self) -> String {
            self.consume_while(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => true,
                _ => false,
            })
        }

        fn parse_node(&mut self) -> Node {
            match self.next_char() {
                '<' => self.parse_element(),
            }
        }

        fn parse_text(&mut self) -> Node {
            text(self.consume_while(|c| c != '<'))
        }

        fn parse_element(&mut self) -> Node {
            assert!(self.consume_char() == '<');
            let tag_name = self.parse_tag_name();
            let attrs = self.parse_attributes();
            assert!(self.consume_char() == '>');

            let children = self.parse_nodes();

            assert!(self.consume_char() == '<');
            assert!(self.consume_char() == '/');
            assert!(self.parse_tag_name() == tag_name);
            assert!(self.consume_char() == '>');

            return elem(tag_name, attrs, children);
        }

        fn parse_attr(&mut self) -> (String, String) {
            let name = self.parse_tag_name();
            assert!(self.consume_char() == '=');
            let value = self.parse_attr_value();
            return (name, value);
        }

        fn parse_attr_value(&mut self) -> String {
            let open_quote = self.consume_char();
            assert!(open_quote == '"' || open_quote == '\'');
            let value = self.consume_while(|c| c != open_quote);
            assert!(self.consume_char() == open_quote);
            return value;
        }

        fn parse_attributes(&mut self) -> AttrMap {
            let mut attributes = HashMap::new();

            loop {
                self.consume_whitespace();

                if self.next_char() == '>' {
                    break;
                }

                let (name, value) = self.parse_attr();
                attributes.insert(name, value);
            }
            return attributes;
        }

        fn parse_nodes(&mut self) -> Vec<Node> {
            let mut nodes = Vec::new();
            loop {
                self.consume_whitespace();
                if self.eof() || self.starts_with("</") {
                    break;
                }
                nodes.push(self.parse_node());
            }
            return nodes;
        }

        fn parse_simple_selector(&mut self) -> SimpleSelector {
            let mut selector = SimpleSelector {
                tag_name: None,
                id: None,
                class: Vec::new(),
            };

            while !self.eof() {
                match self.next_char() {
                    '#' => {
                        self.consume_char();
                        selector.id = Some(self.parse_identifier());
                    }
                    '*' => {
                        self.consume_char();
                    }
                    c if valid_identifier_char(c) => {
                        selector.tag_name = Some(self.parse_identifier());
                    }
                    _ => break,
                }
            }
            return selector;
        }

        fn parse_rule(&mut self) -> Rule {
            Rule {
                selectors: self.parse_selectors(),
                declarations: self.parse_declarations(),
            }
        }

        fn parse_selector(&mut self) -> Vec<Selector> {
            let mut selectors = Vec::new();
            loop {
                selectors.push(Selector::Simple(self.parse_simple_selector()));

                self.consume_whitespace();
                match self.next_char() {
                    ',' => {
                        self.consume_char();
                        self.consume_whitespace();
                    }
                    '{' => break,
                    c => panic!("Unexpected character {} in selector list", c),
                }
            }

            selectors.sort_by(|a, b| {
                return b.specificity().cmp(&a.specificity());
            });

            return selectors;
        }
    }

    struct Node {
        children: Vec<Node>,
        node_type: NodeType,
    }

    enum NodeType {
        Text(String),
        Element(ElementData),
    }

    struct ElementData {
        tag_name: String,
        attributes: AttrMap,
    }

    type AttrMap = HashMap<String, String>;

    fn text(data: String) -> Node {
        Node {
            children: Vec::new(),
            node_type: NodeType::Text(data),
        }
    }

    fn elem(name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
        Node {
            children: children,
            node_type: NodeType::Element(ElementData {
                tag_name: name,
                attributes: attrs,
            }),
        }
    }

    fn source(source: String) -> Node {
        let mut nodes = Parser {
            pos: 0,
            input: source,
        }
        .parse_nodes();

        if nodes.len() == 1 {
            nodes.swap_remove(0)
        } else {
            elem("html".to_string(), HashMap::new(), nodes)
        }
    }
}
