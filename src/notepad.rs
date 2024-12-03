use crate::diff::{Diff, MessageBuf, Operation};

#[derive(Debug, Default)]
pub struct Notepad {
    pub text: String,
}

impl Notepad {
    pub fn apply_message_buf(&mut self, msg: &MessageBuf) {
        msg.messages.iter().for_each(|d| self.apply_diff(d) );
    }

    pub fn apply_diff(&mut self, diff: &Diff) {
        let Diff { opcode, operand, index } = diff;
        let index = *index as usize;

        match opcode {
            Operation::Del => {
                self.remove(index);
            },
            Operation::Ins => {
                self.insert(index, operand.expect("Char not given to Operation: Insert"));
            },
            Operation::Rep => {
                self.replace(index, operand.expect("Char not given to Operation: Rep"));
            }
        }
    }

    fn insert(&mut self, index: usize, value: char) {
        self.text.insert(index, value);
    }

    fn remove(&mut self, index: usize) {
        self.text.remove(index);
    }

    fn replace(&mut self, index: usize, value: char) {
        self.remove(index);
        self.insert(index, value);
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn del_diff() {
        let text = "1: This is my notepad\n2: The next line".to_string();
        let mut notepad = Notepad { text }; 

        let diff = Diff { opcode: Operation::Del, operand: None, index: 2 };

        notepad.apply_diff(&diff);

        assert_eq!(&notepad.text, "1:This is my notepad\n2: The next line")
    }

    #[test]
    fn ins_diff() {
        let text = "1: This is my notepad\n2: The next line".to_string();
        let mut notepad = Notepad { text }; 

        let diff = Diff { opcode: Operation::Ins, operand: Some('\n'), index: 2 };

        notepad.apply_diff(&diff);

        assert_eq!(&notepad.text, "1:\n This is my notepad\n2: The next line")
    }

    #[test]
    fn rep_diff() {
        let text = "1: This is my notepad\n2: The next line".to_string();
        let mut notepad = Notepad { text };

        let diff = Diff { opcode: Operation::Rep, operand: Some('3'), index: 22 };

        notepad.apply_diff(&diff);

        assert_eq!(&notepad.text, "1: This is my notepad\n3: The next line");  
    }
}