use super::Ray;

pub struct Plane {
    pub center: glam::Vec3,
    pub normal: glam::Vec3,
}

impl Plane {
    pub fn intersect(&self, ray: &Ray) -> Option<f32> {
        let denominator = self.normal.dot(ray.dir);

        if denominator.abs() > 0.0001 {
            let difference = self.center - ray.origin;
            let t = difference.dot(self.normal) / denominator;

            if t > 0.0001 {
                return Some(t);
            }
        }

        None
    }
}
