extern crate futures;
use futures::stream::{self, Stream};

pub fn select_all<I, T, E>(streams: I) -> Box<Stream<Item = T, Error = E> + Send>
    where I: IntoIterator,
          I::Item: Stream<Item = T, Error = E> + Send + 'static,
          T: 'static + Send,
          E: 'static + Send,
{
    struct Level<T, E> {
        power: usize,
        stream: Box<Stream<Item = T, Error = E> + Send>,
    }

    let mut stack: Vec<Level<T, E>> = Vec::new();
    for stream in streams {
        let mut lev_a = Level { power: 0, stream: Box::new(stream), };
        while stack.last().map(|l| lev_a.power == l.power).unwrap_or(false) {
            let lev_b = stack.pop().unwrap();
            lev_a = Level {
                power: lev_b.power + 1,
                stream: Box::new(lev_b.stream.select(lev_a.stream)),
            }
        }
        stack.push(lev_a);
    }

    if let Some(tree_lev) = stack.pop() {
        let mut tree = tree_lev.stream;
        while let Some(node) = stack.pop() {
            tree = Box::new(tree.select(node.stream))
        }
        tree
    } else {
        Box::new(stream::empty())
    }
}

#[cfg(test)]
mod tests {
    use futures::{stream, Stream, Future};
    use super::select_all;

    #[test]
    fn happy_path() {
        let stream_a = stream::iter_result(vec![Ok(0), Ok(1)]);
        let stream_b = stream::iter_result(vec![Ok(2), Ok(3), Ok(4)]);
        let stream_c = stream::iter_result(vec![Ok(5)]);

        let mut values = select_all::<_, _, ()>(vec![stream_a, stream_b, stream_c])
            .collect()
            .wait()
            .unwrap();
        values.sort();
        assert_eq!(values, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn an_error() {
        let stream_a = stream::iter_result(vec![Ok(0), Ok(1)]);
        let stream_b = stream::iter_result(vec![Ok(2), Err("fail"), Ok(4)]);
        let stream_c = stream::iter_result(vec![Ok(5)]);

        let status = select_all(vec![stream_a, stream_b, stream_c])
            .collect()
            .wait();
        assert_eq!(status, Err("fail"));
    }
}
