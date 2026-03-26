use serde::{Deserialize, Serialize};

pub struct ParsedHtml {
    pub children:Vec<HtmlNode>
}

impl ParsedHtml {
    pub fn has_elements(&self) -> bool {
        for child in &self.children {
            if let HtmlNode::Element { name, content, children } = child {
                return true
            }
        }
        false
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum HtmlNode {
    Text(String),
    Element{
        name:String,
        content:String,
        children:Vec<HtmlNode>
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum ParserState {
    Start,
    ReadingText,
    ParsingTagName{name:Vec<char>},
    SearchingTagNameEnd{tag_name:Vec<char>, total_len:usize, open_counter:usize},
    SearchingTagEnd{tag_name:Vec<char>, total_start_len:usize, open_counter:usize},
    ParsingTagClose{tag_name:Vec<char>, total_start_len:usize, open_counter:usize},
    ParsingInsideTagOpening{tag_name:Vec<char>, total_start_len:usize, open_counter:usize, actual_name:Vec<char>},
    ParsingTagEnd{tag_name:Vec<char>, actual_name:Vec<char>, total_start_len:usize, open_counter:usize},
}

impl ParserState {
    pub fn next_state(self, cur_char:char) -> Self {
        match self {
            Self::Start => match cur_char {
                '<' => Self::ParsingTagName{name:vec![]},
                _ => Self::ReadingText
            },
            Self::ReadingText => match cur_char {
                '<' => Self::ParsingTagName{name:vec![]},
                _ => Self::ReadingText
            },
            Self::ParsingTagName{name:current_name} => match cur_char {
                '>' => Self::SearchingTagEnd{total_start_len:current_name.len(), tag_name:current_name, open_counter:0},
                ' ' => Self::SearchingTagNameEnd{total_len:current_name.len(), tag_name:current_name, open_counter:0},
                '\n' => Self::Start,
                _ => Self::ParsingTagName{name:add_to_vec(current_name, cur_char)}
            },
            Self::SearchingTagNameEnd { tag_name, total_len, open_counter } => match cur_char {
                '>' => Self::SearchingTagEnd { tag_name, total_start_len:total_len, open_counter },
                '\n' => if open_counter > 0 {
                    Self::SearchingTagEnd { tag_name, total_start_len:total_len, open_counter }
                }
                else {
                    Self::Start
                },
                _ => Self::SearchingTagNameEnd { tag_name, total_len:total_len + 1, open_counter }
            }
            Self::SearchingTagEnd { tag_name, total_start_len, open_counter } => match cur_char {
                '<' => Self::ParsingTagClose { tag_name, total_start_len, open_counter },
                _ => Self::SearchingTagEnd { tag_name, total_start_len, open_counter }
            },
            Self::ParsingTagClose { tag_name, total_start_len, open_counter } => match cur_char {
                '/' => Self::ParsingTagEnd { tag_name, actual_name: vec![], total_start_len, open_counter },
                _ => Self::ParsingInsideTagOpening { tag_name, total_start_len, open_counter, actual_name: vec![cur_char] }
            }
            Self::ParsingTagEnd { tag_name, actual_name, total_start_len, open_counter } => match cur_char {
                '>' => if tag_name == actual_name {
                    if open_counter == 0 {
                        Self::Start
                    }
                    else {
                        Self::SearchingTagEnd { tag_name, total_start_len, open_counter:open_counter-1 }
                    }
                    // must also handle newlines
                }
                else {
                    Self::SearchingTagEnd { tag_name, total_start_len, open_counter }
                }
                ' ' => Self::SearchingTagEnd { tag_name, total_start_len, open_counter },
                _ => Self::ParsingTagEnd { tag_name, actual_name:add_to_vec(actual_name, cur_char), total_start_len, open_counter }
            },
            Self::ParsingInsideTagOpening { tag_name, total_start_len, open_counter, actual_name } => match cur_char {
                '>' => if tag_name == actual_name {
                    // something fishy here because of the space
                    Self::SearchingTagEnd { tag_name, total_start_len, open_counter:open_counter+1 }
                }
                else {
                    Self::SearchingTagEnd { tag_name, total_start_len, open_counter }
                },
                ' ' => if tag_name == actual_name {
                    Self::SearchingTagNameEnd { tag_name, total_len: total_start_len, open_counter:open_counter + 1 }
                }
                else {
                    Self::SearchingTagEnd { tag_name, total_start_len, open_counter }
                },
                _ =>  Self::ParsingInsideTagOpening { tag_name, total_start_len, open_counter, actual_name:add_to_vec(actual_name, cur_char) }
                
            }

        }
    }
    pub fn important_change(&self, new_state:&Self) -> bool {
        match self {
            Self::Start => false,
            Self::ReadingText => match new_state {
                Self::ReadingText => false,
                _ => true,
            },
            Self::ParsingTagEnd { tag_name, actual_name,total_start_len, open_counter } => match new_state {
                Self::Start => true,
                _ => false,
            },
            _ => false
        }
    }
    pub fn tag_name(&self) -> (Vec<char>, usize) {
        match self {
            Self::ParsingTagClose { tag_name,total_start_len, open_counter } => (tag_name.clone(), *total_start_len),
            Self::ParsingTagEnd { tag_name, actual_name,total_start_len, open_counter } => (tag_name.clone(), *total_start_len),
            Self::SearchingTagEnd { tag_name, total_start_len, open_counter } => (tag_name.clone(), *total_start_len),
            _ => panic!("Impossible")
        }
    }
}

fn add_to_vec(mut vector:Vec<char>, new:char) -> Vec<char> {
    vector.push(new);
    vector
}

fn chars_to_string(chars:&[char]) -> String {
    let mut str = String::with_capacity(chars.len());
    for char in chars {
        str.push(*char);
    }
    str
}

pub fn parse_html(chars:&[char], max_depth:usize) -> ParsedHtml {
    if max_depth > 0 {
        let mut start_ind = 0;
        let mut end_ind = 0;
        if chars.len() > 0 {
            let mut children = Vec::with_capacity(4);
            let mut state = ParserState::Start;
            while end_ind < chars.len() {
                let new_state = state.clone().next_state(chars[end_ind]);
                if state.important_change(&new_state) {
                    if let ParserState::ReadingText = state.clone() {
                        let str = chars_to_string(&chars[start_ind..end_ind]);
                        children.push(HtmlNode::Text(str));
                    }
                    else {
                        let (tag_name, start_tag_len) = state.tag_name();
                        let slice = &chars[(start_ind+start_tag_len+2)..(end_ind-tag_name.len()-3)];
                        let inside_str = chars_to_string(slice);
                        let parsed = parse_html(slice, max_depth-1);
                        children.push(HtmlNode::Element { name: chars_to_string(&tag_name), content: inside_str, children:parsed.children });
                    }
                    start_ind = end_ind + 1;
                }
                state = new_state;
                end_ind += 1;
            }
            if end_ind != start_ind {
                let str = chars_to_string(&chars[start_ind..end_ind]);
                children.push(HtmlNode::Text(str));
            }
            ParsedHtml { children }
        }
        else {
            ParsedHtml { children: vec![] }
        }
    }
    else {
        ParsedHtml { children: vec![] }
    }
}