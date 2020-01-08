//!
//! ## Overview
//!
//! This crate provides the opengl internals without the opengl context creation functionality.
//! So this crate does not depend on glutin.
//!
use crate::shapes::*;
use axgeom::*;

use core::mem;
use gl::types::*;

use crate::sprite_program::*;
use crate::circle_program::*;



type PointType=[f32;2];

mod shader;

///Contains all the texture/sprite drawing code.
///The api is described in the crate documentation.
pub mod sprite;
mod vbo;

///Macro that asserts that there are no opengl errors.
#[macro_export]
macro_rules! gl_ok {
    () => {
        assert_eq!(gl::GetError(), gl::NO_ERROR);
    };
}
struct NotSend(*mut usize);

fn ns() -> NotSend {
    NotSend(core::ptr::null_mut())
}

use circle_program::CircleProgram;
use circle_program::PointMul;
mod circle_program;
use sprite_program::SpriteProgram;
mod sprite_program;

///All the opengl functions generated from the gl_generator crate.
pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

///Contains all the shape drawing session and save objects
///They all follow the same api outlined in the crate documentation.
pub mod shapes;


use self::uniforms::*;
///Contains the objects used for the uniform setting stage of the egaku2d drawing pipeline.
pub mod uniforms{
    use super::*;
    use vbo::BufferInfo;

    pub struct StaticUniforms<'a>{
        pub(crate) sys:&'a mut SimpleCanvas,
        pub(crate) un:ProgramUniformValues,
        pub(crate) buffer:BufferInfo
    }
    impl StaticUniforms<'_>{
        pub fn with_color(&mut self,color:[f32;4])->&mut Self{
            self.un.color=color;
            self
        }

        pub fn draw(&mut self){
            self.sys.circle_program.set_buffer_and_draw(
                &self.un,
                self.buffer
            );
        }
        //TOO add offset
    }


    pub struct StaticSpriteUniforms<'a>{
        pub(crate) sys:&'a mut SimpleCanvas,
        pub(crate) un:SpriteProgramUniformValues<'a>,
        pub(crate) buffer:BufferInfo
    }

    impl StaticSpriteUniforms<'_>{
        pub fn with_color(&mut self,color:[f32;4])->&mut Self{
            self.un.color=color;
            self
        }

        pub fn draw(&mut self){
            self.sys.sprite_program.set_buffer_and_draw(
                &self.un,
                self.buffer,
            );
        }
    }

    pub struct SpriteUniforms<'a>{
        pub(crate) sys:&'a mut SimpleCanvas,
        pub(crate) un:SpriteProgramUniformValues<'a>,
    }
    impl SpriteUniforms<'_>{
        pub fn with_color(&mut self,color:[f32;4])->&mut Self{
            self.un.color=color;
            self
        }
        pub fn send_and_draw(&mut self){
            self.sys.sprite_buffer.update();

            self.sys.sprite_program.set_buffer_and_draw(
                &self.un,
                self.sys.sprite_buffer.get_info()
            );
        }
    }


    pub struct Uniforms<'a>{
        pub(crate) sys:&'a mut SimpleCanvas,
        pub(crate) un:ProgramUniformValues,
    }

    impl Uniforms<'_>{
        pub fn with_color(&mut self,color:[f32;4])->&mut Self{
            self.un.color=color;
            self
        }

        pub fn send_and_draw(&mut self){
            self.sys.circle_buffer.update();

            self.sys.circle_program.set_buffer_and_draw(
                &self.un,
                self.sys.circle_buffer.get_info()
            );
        }
        //TODO add offset
    }

}



///Allows the user to start drawing shapes.
///The top left corner is the origin.
///y grows as you go down.
///x grows as you go right.
pub struct SimpleCanvas {
    _ns: NotSend,
    circle_program: CircleProgram,
    sprite_program: SpriteProgram,
    point_mul: PointMul,


    //It is important to note that this buffers might not be empty when a session object is dropped.
    //the buffers are cleared on creation of a session.
    //this allows us to not have to implement Drop for the session to make sure that the buffer is cleared.
    //if they were to implement drop, they would be slightly less egronomic to use.
    circle_buffer: vbo::GrowableBuffer<circle_program::Vertex>,
    sprite_buffer: vbo::GrowableBuffer<sprite_program::Vertex>,
    color:[f32;4] //Default color used
}

impl SimpleCanvas {
    pub fn set_default_color(&mut self,color:[f32;4]){
        self.color=color;
    }

    pub fn set_viewport(&mut self, window_dim: axgeom::FixedAspectVec2, game_width: f32) {
        self.point_mul = self.circle_program.set_viewport(window_dim, game_width);

        let _ = self.sprite_program.set_viewport(window_dim, game_width);
    }

    //Unsafe since user might create two instances, both of
    //which could make opengl calls simultaneously
    pub unsafe fn new(window_dim: axgeom::FixedAspectVec2) -> SimpleCanvas {
        let circle_buffer = vbo::GrowableBuffer::new();
        let sprite_buffer = vbo::GrowableBuffer::new();

        let mut circle_program = CircleProgram::new();

        let mut sprite_program = SpriteProgram::new();

        let point_mul = circle_program.set_viewport(window_dim, window_dim.width as f32);
        let _ = sprite_program.set_viewport(window_dim, window_dim.width as f32);

        gl::Enable(gl::BLEND);
        gl_ok!();
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl_ok!();

        SimpleCanvas {
            _ns: ns(),
            point_mul,
            sprite_program,
            circle_program,
            circle_buffer,
            sprite_buffer,
            color:[1.0;4]
        }
    }

    pub fn sprites(&mut self) -> sprite::SpriteSession {
        self.sprite_buffer.clear();
        sprite::SpriteSession { sys: self }
    }

    pub fn circles(&mut self) -> CircleSession {
        self.circle_buffer.clear();
        CircleSession { sys: self }
    }
    pub fn squares(&mut self) -> SquareSession {
        self.circle_buffer.clear();
        SquareSession { sys: self }
    }
    pub fn rects(&mut self) -> RectSession {
        self.circle_buffer.clear();
        RectSession { sys: self }
    }
    pub fn arrows(&mut self, radius: f32) -> ArrowSession {
        self.circle_buffer.clear();
        let kk = self.point_mul.0;

        ArrowSession {
            sys: self,
            radius: radius * kk,
        }
    }

    pub fn lines(&mut self, radius: f32) -> LineSession {
        self.circle_buffer.clear();
        let kk = self.point_mul.0;
        LineSession {
            sys: self,
            radius: radius * kk,
        }
    }



    pub fn clear_color(&mut self, back_color: [f32; 3]) {
        unsafe {
            gl::ClearColor(back_color[0], back_color[1], back_color[2], 1.0);
            gl_ok!();

            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl_ok!();
        }
    }
}
