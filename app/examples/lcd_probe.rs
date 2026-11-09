//! Export the production icon coverage for comparison with Qt's original SVG renderer.
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../src/lcd_masks.rs"] mod lcd_masks;
#[allow(dead_code)]
#[path = "../src/lcd_render.rs"]
mod lcd_render;
use retrohatch::core::profile::DeviceProfile;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    let p = DeviceProfile::parse(&a[0]).ok_or("profile")?;
    let scale: f32 = a[1].parse()?;
    let root = std::path::Path::new(&a[2]);
    std::fs::create_dir_all(root)?;
    let source = lcd_render::masks(p);
    for (i, icon) in source.icons.iter().enumerate() {
        let c = lcd_render::coverage(source.data, icon, scale);
        let mut bytes = format!("P5\n{} {}\n255\n", c.width, c.height).into_bytes();
        bytes.extend(c.alpha);
        std::fs::write(root.join(format!("{i}.pgm")), bytes)?;
        println!("{i} {} {} {} {}", c.left, c.top, c.width, c.height);
    }
    Ok(())
}
