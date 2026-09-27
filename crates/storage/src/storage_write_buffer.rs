pub struct StorageWriteBuffer<T> {
    buffer: Vec<T>,
    capacity: usize,
}

impl<T> StorageWriteBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.buffer.push(item);
        if self.buffer.len() >= self.capacity {
            Some(std::mem::replace(
                &mut self.buffer,
                Vec::with_capacity(self.capacity),
            ))
        } else {
            None
        }
    }

    pub fn flush(&mut self) -> Option<Vec<T>> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(std::mem::replace(
                &mut self.buffer,
                Vec::with_capacity(self.capacity),
            ))
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_write_buffer_push_and_flush() {
        let mut buf = StorageWriteBuffer::new(3);
        assert_eq!(buf.push(1), None);
        assert_eq!(buf.push(2), None);
        assert_eq!(buf.push(3), Some(vec![1, 2, 3]));
        assert_eq!(buf.len(), 0);

        assert_eq!(buf.push(4), None);
        assert_eq!(buf.flush(), Some(vec![4]));
        assert_eq!(buf.flush(), None);
    }
}
