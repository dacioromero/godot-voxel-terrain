use gdnative::api::{ArrayMesh, Mesh, MeshInstance, OpenSimplexNoise, SurfaceTool};
use gdnative::prelude::*;
use marching_cubes::{polygonise_vec, POINTS};
use std::convert::TryInto;
use std::sync::mpsc::channel;
use std::sync::Arc;
use threadpool::ThreadPool;

type Vector3Int = euclid::default::Vector3D<i32>;

const C_SIZE: i32 = 64;
const C_C_SIZE: i32 = C_SIZE * C_SIZE * C_SIZE;
// const C_C_SIZE: i32 = C_SIZE.pow(3);

fn idx_to_vec3int(idx: usize) -> Vector3Int {
    let z: i32 = idx as i32 % C_SIZE;
    let y: i32 = (idx as i32 / C_SIZE) % C_SIZE;
    let x: i32 = idx as i32 / (C_SIZE * C_SIZE);

    return Vector3Int::new(x, y, z);
}

fn vec3int_to_idx(vec3: Vector3Int) -> usize {
    let idx = vec3.z + C_SIZE * (vec3.y + C_SIZE * vec3.x);

    idx as usize
}

#[derive(NativeClass)]
#[inherit(MeshInstance)]
pub struct VoxelTerrain;
// #[register_with(register_properties)]
// pub struct VoxelTerrain {
//     scale: f32,
// }

// fn register_properties(builder: &ClassBuilder<VoxelTerrain>) {
//     builder
//         .add_property::<f32>("Hi")
//         .with_default(4.0)
//         .with_getter(|vt: &VoxelTerrain, _| vt.scale)
//         .with_setter(|vt: &mut VoxelTerrain, _o: &MeshInstance, scale: f32| {
//             vt.scale = scale;
//         })
//         .done();
// }

#[methods]
impl VoxelTerrain {
    fn new(_owner: &MeshInstance) -> Self {
        VoxelTerrain
        // VoxelTerrain { scale: 0.0 }
    }

    #[export]
    fn _process(&self, owner: &MeshInstance, _delta: f64) {
        let input = Input::godot_singleton();

        if input.is_action_just_pressed("generate") {
            self.generate(owner);
        }
    }

    #[export]
    fn _ready(&self, owner: &MeshInstance) {
        self.generate(owner);
    }

    fn generate(&self, owner: &MeshInstance) {
        let noise: Ref<OpenSimplexNoise, Unique> = OpenSimplexNoise::new();
        let mut voxels: [f32; C_C_SIZE as usize] = [0.0; C_C_SIZE as usize];

        for (i, voxel) in voxels.iter_mut().enumerate() {
            let pos: Vector3 = idx_to_vec3int(i).to_f32() * 4.0;
            *voxel = (noise.get_noise_3dv(pos) as f32 + 1.0) / 2.0;
        }

        let voxels = Arc::new(voxels);
        let pool: ThreadPool = Default::default();
        let (tx, rx) = channel();

        for x in 0..C_SIZE - 1 {
            for y in 0..C_SIZE - 1 {
                for z in 0..C_SIZE - 1 {
                    let voxels = Arc::clone(&voxels);
                    let tx = tx.clone();

                    pool.execute(move || {
                        let offset = Vector3Int::new(x, y, z);
                        let values = POINTS
                            .iter()
                            .map(|p| p.to_i32() + offset)
                            .map(vec3int_to_idx)
                            .map(|i| voxels[i])
                            .collect::<Vec<_>>()
                            .as_slice()
                            .try_into()
                            .unwrap();
                        let triangles = polygonise_vec(&values, 0.5);

                        tx.send((triangles, offset)).unwrap();
                    });
                }
            }
        }

        let vertices = rx
            .iter()
            .take((C_SIZE as usize - 1).pow(3))
            .flat_map(|(ts, o)| ts.into_iter().map(move |t| t + o.to_f32()))
            .flatten();
        let surface_tool: Ref<SurfaceTool, Unique> = SurfaceTool::new();

        surface_tool.begin(Mesh::PRIMITIVE_TRIANGLES);
        surface_tool.add_smooth_group(true);

        for vertex in vertices {
            surface_tool.add_vertex(vertex);
        }

        surface_tool.index();
        surface_tool.generate_normals(false);

        let mesh: Ref<ArrayMesh> = surface_tool
            .commit(Null::null(), Mesh::ARRAY_COMPRESS_DEFAULT)
            .expect("Failed to create mesh");

        owner.set_mesh(mesh);
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<VoxelTerrain>();
}

godot_init!(init);
