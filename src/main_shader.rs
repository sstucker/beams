// TODO https://www.youtube.com/watch?v=neyIpnII-WQ

use std::borrow::Cow;

use bevy::{prelude::*, render::{*, render_resource::*, texture::*, extract_component::{ExtractComponentPlugin, ExtractComponent, self}, render_graph::RenderGraph, renderer::{RenderContext, RenderDevice}, render_asset::RenderAssets}, utils::HashMap};

const PARTICLE_COUNT: u32 = 1;
const WORKGROUP_SIZE: u32 = 4;
const WIDTH: f32 = 1024.;
const HEIGHT: f32 = 1024.;

fn create_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = Image::new_fill(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1
        },
        TextureDimension::D3,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8Unorm
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST |
        TextureUsages::STORAGE_BINDING |
        TextureUsages::TEXTURE_BINDING;
    image.sampler_descriptor = ImageSampler::nearest();
    images.add(image)
}

#[derive(Resource, Clone)]
pub struct ParticleUpdatePipeline {
    bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId
}

fn update_bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
    BindGroupLayoutDescriptor {
        label: None,
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None
            },
            count: None
        }]
    }
}

impl FromWorld for ParticleUpdatePipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout: BindGroupLayout = world
            .resource::<renderer::RenderDevice>()
            .create_bind_group_layout(&update_bind_group_layout());
        let shader = world.resource::<AssetServer>().load("particle_update.wgsl");
        let mut pipeline_cache: Mut<PipelineCache> = world.resource_mut::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
            shader.clone(),
            "init",
            &bind_group_layout
        ));

        let update_pipeline = pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
            shader,
            "update",
            &bind_group_layout
        ));

        ParticleUpdatePipeline {
            bind_group_layout,
            init_pipeline,
            update_pipeline
        }
    }
}

pub fn compute_pipeline_descriptor(
    shader: Handle<Shader>,
    entry_point: &str,
    bind_group_layout: &BindGroupLayout
) -> ComputePipelineDescriptor {
    ComputePipelineDescriptor {
        label: None,
        layout: Some(vec![bind_group_layout.clone()]),
        shader,
        shader_defs: vec![],
        entry_point: Cow::from(entry_point.to_owned())
    }
}

pub fn update_bind_group(
    entity: Entity,
    render_device: &RenderDevice,
    update_pipeline: &ParticleUpdatePipeline,
    particle_system_render: &ParticleSystemRender
) -> BindGroup {
    render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &update_pipeline.bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer((particle_system_render.particle_buffers[&entity].as_entire_buffer_binding()))
        }]
    })
}

#[derive(Resource, Default)]
pub struct ParticleSystemRender {
    pub update_bind_group: HashMap<Entity, BindGroup>,
    pub render_bind_group: HashMap<Entity, BindGroup>,
    pub particle_buffers: HashMap<Entity, Buffer>
}

pub fn run_compute_pass(
    render_context: &mut RenderContext,
    bind_group: &BindGroup,
    pipeline_cache: &PipelineCache,
    pipeline: CachedComputePipelineId
) {
    let mut pass: ComputePass = render_context.command_encoder
        .begin_compute_pass(&ComputePassDescriptor::default());
    pass.set_bind_group(0, bind_group, &[]);
    let pipeline = pipeline_cache.get_compute_pipeline(pipeline).unwrap();
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(PARTICLE_COUNT / WORKGROUP_SIZE, 1, 1)
}

#[derive(Default, Clone)]
enum ParticleUpdateState {
    #[default]
    Loading,
    Init,
    Update
}

#[derive(Resource, Default)]
pub struct ParticleSystemRender {
    pub update_bind_group: HashMap<Entity, BindGroup>,
    pub render_bind_group: HashMap<Entity, BindGroup>,
    pub particle_buffers: HashMap<Entity, Buffer>
}

fn queue_bind_group(
    render_device: Res<RenderDevice>,
    render_pipeline: Res<ParticleRenderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    mut particle_system_render: ResMut<ParticleSystemRender>,
    update_pipeline: Res<ParticleUpdatePipeline>,
    particle_Systems: Query<(Entity, &ParticleSystem)>
) {
    for (entity, system) in &particle_Systems {
        if !particle_system_render.particle_buffers.contains_key(&entity) {
            let particle = [Particle::default(); PARTICLE_COUNT as usize];
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::StorageBuffer::new(&mut byte_buffer);
            buffer.write(&particle).unwrap();

            let storage = render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: None,
                    usage:
                        BufferUsages::COPY_DST |
                        BufferUsages::STORAGE |
                        BufferUsages::COPY_SRC,
                    contents: buffer.into_inner()
                }
            );
            particle_system_render.particle_buffers.insert(entity, storage);
        }
        if !particle_system_render.update_bind_group.contains_key(&entity) {
            let update_group = update_bind_group(entity, &render_device, &update_pipeline, &particle_system_render);
            particle_system_render.update_bind_group.insert(entity, update_group);
        }
    }
}

#[derive(ShaderType, Default, Clone, Copy)]
struct Particle {
    position: Vec2
}

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<ParticleSystem>::default());
    
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ParticleUpdatePipeline>()
            .init_resource::<ParticleSystemRender>()
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

            let mut render_graph = render_app.world.resource_mut::<render_graph::RenderGraph>();
            let mut update_node = UpdateParticlesNode::new(&mut render_app.world);
            render_graph.add_node("update_particles", update_node);
            render_graph.add_node_edge(
                "update_particles", 
                main_graph::node::CAMERA_DRIVER,
            ).unwrap();
    }
} 

// ParticleSystem

#[derive(Default, Component, Clone)]
pub struct ParticleSystem {
    pub rendered_texture: Handle<Image>
}

impl extract_component::ExtractComponent for ParticleSystem {
    type Query = &'static ParticleSystem;
    type Filter = ();
    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Self {
        item.clone()
    }
}

// UpdateParticlesNode

pub struct UpdateParticlesNode {
    particle_systems: QueryState<Entity, With<ParticleSystem>>,
    state_map: HashMap<Entity, ParticleUpdateState>
}

impl UpdateParticlesNode {
    fn update_state(
        &mut self,
        entity: Entity,
        pipeline_cache: &PipelineCache,
        pipeline: &ParticleUpdatePipeline
    ) {
        let update_state: &ParticleUpdateState = match self.state_map.get(&entity) {
            Some(state) => state,
            None => {
                self.state_map.insert(entity, ParticleUpdateState::Loading);
                &ParticleUpdateState::Loading
            }
        };
        match update_state {
            ParticleUpdateState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                    {
                        self.state_map.insert(entity, ParticleUpdateState::Init);
                    }
            }
            ParticleUpdateState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                    {
                        self.state_map.insert(entity, ParticleUpdateState::Update);
                    }
            }
            ParticleUpdateState::Update => {}
        }
    }
}

impl render_graph::Node for UpdateParticlesNode {
    
    fn update(&mut self, world: &mut World) {
        let mut systems
            = world.query_filtered::<Entity, With<ParticleSystem>>();
        let pipeline
            = world.resource::<ParticleUpdatePipeline>();
        let pipeline_cache
            = world.resource::<PipelineCache>();
        for entity in systems.iter(world) {
            self.update_state(entity, pipeline_cache, pipeline);
        }
        self.particle_systems.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline
        = world.resource::<ParticleUpdatePipeline>();
        let pipeline_cache
            = world.resource::<PipelineCache>();
        let particle_systems_renderer = world.resource::<ParticleSystemRender>();

        for entity in self.particle_systems.iter_manual(world) {
            if let Some(pipeline) = match self.state_map[&entity] {
                ParticleUpdateState::Loading => None,
                ParticleUpdateState::Init => Some(pipeline.init_pipeline),
                ParticleUpdateState::Update => Some(pipeline.update_pipeline)
            } {
                run_compute_pass(
                    render_context,
                    &particle_systems_renderer.update_bind_group[&entity],
                    pipeline_cache,
                    pipeline
                );
            }
        }
        Ok(())
    }

}

fn main() {
    let mut app: App = App::new();
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: WIDTH,
                height: HEIGHT,
                title: "Particles".to_string(),
                resizable: false,
                ..Default::default()
            },
            ..Default::default()
        }))
 
        .add_plugin(ParticlePlugin)
        .add_startup_system(setup)
        .add_system(spawn_on_space_bar);
        .run();
    println!("Hello, world!");
}
