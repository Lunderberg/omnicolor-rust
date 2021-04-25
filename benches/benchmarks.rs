// -- Cargo.toml --
// [[bench]]
// name = "benchmarks"
// harness = false
//
// [dev-dependencies]
// criterion = {version = "0.3", features=['html_reports']}

use criterion::{
    black_box, criterion_group, criterion_main, Bencher, Criterion,
};

use omnicolor_rust::palettes::generate_uniform_palette;
use omnicolor_rust::GrowthImageBuilder;

fn generate_flat_image(b: &mut Bencher) {
    let width = black_box(1920);
    let height = black_box(1080);
    let epsilon = black_box(5.0);

    let palette = generate_uniform_palette(width * height);

    b.iter(|| {
        let mut image = GrowthImageBuilder::new(width, height)
            .epsilon(epsilon)
            .palette(palette.clone())
            .build()
            .unwrap();
        while !image.is_done() {
            image.fill();
        }
    });
}

fn bench_flat_image(c: &mut Criterion) {
    let mut group = c.benchmark_group("Image-gen");
    group
        .noise_threshold(0.07)
        .sample_size(20)
        .sampling_mode(criterion::SamplingMode::Flat)
        .measurement_time(std::time::Duration::from_secs(120));

    group.bench_function("flat-image", generate_flat_image);

    group.finish();
}

criterion_group!(benches, bench_flat_image);
criterion_main!(benches);
