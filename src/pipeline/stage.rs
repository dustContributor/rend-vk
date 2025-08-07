use ash::vk;

pub trait Stage {
    fn name(&self) -> &str;
    fn index(&self) -> u32;
    fn is_validation_layer_enabled(&self) -> bool;
    fn image_barriers(&self) -> Vec<vk::ImageMemoryBarrier2>;
    fn destroy(&self, device: &ash::Device);
    fn work(&mut self, ctx: super::RenderContext);
}
