# Aircraft assets

The airplane is generated entirely in `src/aircraft.rs` from Bevy sphere and cuboid meshes. No external aircraft model, texture, network download or third-party aircraft license is required. The parent transform is the body reference (+X right, +Y up, -Z nose); the propeller is an independently animated child.

The bundled Noto Sans font in `../fonts/` is preserved from the existing project and distributed under the accompanying SIL Open Font License (`../fonts/OFL.txt`).
