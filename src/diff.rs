#[derive(Debug, PartialEq)]
pub struct Diff {
    pub opcode: Operation,
    pub operand: Option<char>,
    pub index: u8,
}

#[derive(Debug, Default, PartialEq)]
pub struct MessageBuf {
    pub messages: Vec<Diff>,
}

#[derive(Debug, PartialEq)]
pub enum Operation {
    Del,
    Ins,
    Rep
}

impl TryFrom<u8> for Operation {
    type Error = &'static str;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Operation::Del),
            1 => Ok(Operation::Ins),
            2 => Ok(Operation::Rep),
            _ => Err("Invalid opcode byte")
        }
    }
    
}

impl From<Vec<u8>> for MessageBuf {
    fn from(data: Vec<u8>) -> Self {
        let mut messages = Vec::new();

        if data.len() % 3 != 0 {
            panic!("Data length must be a multiple of 3");
        }

        for chunk in data.chunks(3) {
            let opcode = chunk[0].try_into().unwrap(); 

            let operand = match chunk[1] {
                0 => None,
                c => Some(c as char)
            };

            let index = chunk[2];

            messages.push(Diff { opcode, operand, index });
        }

        MessageBuf { messages }
    }
}

impl Into<Vec<u8>> for MessageBuf {
    fn into(self) -> Vec<u8> {
        let mut data = Vec::new();
        for Diff { opcode, operand, index } in self.messages {
            let opcode_byte = opcode as u8;
            let operand_byte = match operand {
                Some(c) => c as u8,
                None => 0,
            };
            data.push(opcode_byte);
            data.push(operand_byte);
            data.push(index);
        }

        data
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn byte_to_operation() {
        assert_eq!(0.try_into(), Ok(Operation::Del));
        assert_eq!(1.try_into(), Ok(Operation::Ins));
        assert_eq!(2.try_into(), Ok(Operation::Rep));
        let e: Result<Operation, _> = 3.try_into();
        assert!(e.is_err());
    }

    #[test]
    fn byte_from_operation() {
        assert_eq!(Operation::Del as u8, 0);
        assert_eq!(Operation::Ins as u8, 1);
        assert_eq!(Operation::Rep as u8, 2);
    }

    fn def_message() -> MessageBuf {
        MessageBuf { 
            messages: vec![
                Diff { opcode: Operation::Ins, operand: Some('a'), index: 0 },
                Diff { opcode: Operation::Ins, operand: Some('b'), index: 0 },
                Diff { opcode: Operation::Del, operand: None, index: 1 },
            ] 
        }
    }

    #[test]
    fn into_message_buf() {
        let message = def_message();

        let data: Vec<u8> = message.into();

        assert_eq!(data, vec![1, 97, 0, 1, 98, 0, 0, 0, 1]);
    }
    
    #[test]
    fn from_message_buf() {
        let data = vec![1, 97, 0, 1, 98, 0, 0, 0, 1];

        let message: MessageBuf = data.into();

        assert_eq!(message, def_message());
    }

}