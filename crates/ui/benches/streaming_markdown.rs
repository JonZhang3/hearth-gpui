use criterion::{Criterion, black_box, criterion_group, criterion_main};
use hearth_gpui::text::StreamingTextPacer;

fn streaming_text_pacer(c: &mut Criterion) {
    let source = format!(
        "# Streaming Markdown\n\n{}\n\n```rust\n{}\n```\n",
        "A paragraph with **strong**, `code`, and [links](https://example.com). ".repeat(512),
        "fn render(value: usize) -> usize { value + 1 }\n".repeat(256),
    );

    c.bench_function("streaming_text_pacer_64k", |bench| {
        bench.iter(|| {
            let mut pacer = StreamingTextPacer::new();
            pacer.push_str(black_box(&source));
            let mut emitted = 0;
            while let Some(chunk) = pacer.take_chunk() {
                emitted += black_box(chunk.len());
            }
            black_box(emitted)
        });
    });
}

criterion_group!(benches, streaming_text_pacer);
criterion_main!(benches);
