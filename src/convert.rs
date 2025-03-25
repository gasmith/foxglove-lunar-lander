use foxglove::schemas::{Quaternion, Vector3};

pub trait IntoFg<T> {
    fn into_fg(self) -> T;
}
impl IntoFg<Vector3> for glam::Vec3 {
    fn into_fg(self) -> Vector3 {
        Vector3 {
            x: self.x.into(),
            y: self.y.into(),
            z: self.z.into(),
        }
    }
}
impl IntoFg<Quaternion> for glam::Quat {
    fn into_fg(self) -> Quaternion {
        Quaternion {
            x: self.x.into(),
            y: self.y.into(),
            z: self.z.into(),
            w: self.w.into(),
        }
    }
}
