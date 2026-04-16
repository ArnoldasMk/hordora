use smithay::{
    backend::renderer::{
        element::{
            Element, Id, Kind, RenderElement, UnderlyingStorage,
            memory::MemoryRenderBufferRenderElement,
            render_elements,
            surface::WaylandSurfaceRenderElement,
            texture::TextureRenderElement,
            utils::RescaleRenderElement,
        },
        gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, element::PixelShaderElement},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};

/// Render element that tiles a texture across an area using a custom GLSL shader.
/// Behaves like `PixelShaderElement` for element tracking (stable ID, area-based
/// geometry, resize/update_uniforms) but renders via `render_texture_from_to`
/// so the shader can sample the tile texture.
#[derive(Debug, Clone)]
pub struct TileShaderElement {
    shader: GlesTexProgram,
    texture: GlesTexture,
    pub tex_w: i32,
    pub tex_h: i32,
    id: Id,
    commit_counter: CommitCounter,
    area: Rectangle<i32, Logical>,
    opaque_regions: Vec<Rectangle<i32, Logical>>,
    alpha: f32,
    additional_uniforms: Vec<Uniform<'static>>,
    kind: Kind,
}

impl TileShaderElement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        shader: GlesTexProgram,
        texture: GlesTexture,
        tex_w: i32,
        tex_h: i32,
        area: Rectangle<i32, Logical>,
        opaque_regions: Option<Vec<Rectangle<i32, Logical>>>,
        alpha: f32,
        additional_uniforms: Vec<Uniform<'_>>,
        kind: Kind,
    ) -> Self {
        Self {
            shader,
            texture,
            tex_w,
            tex_h,
            id: Id::new(),
            commit_counter: CommitCounter::default(),
            area,
            opaque_regions: opaque_regions.unwrap_or_default(),
            alpha,
            additional_uniforms: additional_uniforms.into_iter().map(|u| u.into_owned()).collect(),
            kind,
        }
    }

    pub fn resize(
        &mut self,
        area: Rectangle<i32, Logical>,
        opaque_regions: Option<Vec<Rectangle<i32, Logical>>>,
    ) {
        let opaque_regions = opaque_regions.unwrap_or_default();
        if self.area != area || self.opaque_regions != opaque_regions {
            self.area = area;
            self.opaque_regions = opaque_regions;
            self.commit_counter.increment();
        }
    }

    pub fn update_uniforms(&mut self, additional_uniforms: Vec<Uniform<'_>>) {
        self.additional_uniforms = additional_uniforms.into_iter().map(|u| u.into_owned()).collect();
        self.commit_counter.increment();
    }
}

impl Element for TileShaderElement {
    fn id(&self) -> &Id { &self.id }
    fn current_commit(&self) -> CommitCounter { self.commit_counter }

    fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> {
        Rectangle::from_size((self.tex_w as f64, self.tex_h as f64).into())
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.area.to_physical_precise_round(scale)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.opaque_regions
            .iter()
            .map(|region| region.to_physical_precise_round(scale))
            .collect()
    }

    fn alpha(&self) -> f32 { self.alpha }
    fn kind(&self) -> Kind { self.kind }
}

impl RenderElement<GlesRenderer> for TileShaderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        _user_data: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            opaque_regions,
            Transform::Normal,
            self.alpha,
            Some(&self.shader),
            &self.additional_uniforms,
        )
    }

    #[inline]
    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}

/// Corner-rounding helper: scales a pre-zoom physical rect into a post-zoom
/// physical rect by rounding the TWO CORNERS independently (not loc+size).
///
/// Smithay's `Rectangle::to_i32_round()` rounds `loc` and `size` independently,
/// so for non-integer `scale` the resulting `right = round(loc*s) + round(size*s)`
/// can differ from `round((loc+size)*s)` by ±1 physical pixel. That off-by-one
/// is the source of black seams on window bodies at fractional zoom levels.
/// Corner rounding is pixel-consistent: adjacent elements sharing a pre-zoom
/// coordinate always meet at the same post-zoom pixel.
pub fn corner_round_rect(
    rect: Rectangle<f64, Physical>,
    scale: Scale<f64>,
) -> Rectangle<i32, Physical> {
    let x0 = (rect.loc.x * scale.x).round() as i32;
    let y0 = (rect.loc.y * scale.y).round() as i32;
    let x1 = ((rect.loc.x + rect.size.w) * scale.x).round() as i32;
    let y1 = ((rect.loc.y + rect.size.h) * scale.y).round() as i32;
    Rectangle::new(
        Point::from((x0, y0)),
        Size::from(((x1 - x0).max(0), (y1 - y0).max(0))),
    )
}

/// Drop-in replacement for `smithay::backend::renderer::element::utils::RescaleRenderElement`
/// that uses pixel-snapped corner rounding (see [`corner_round_rect`]).
///
/// Used only for window surface elements — shadows and decorations keep smithay's
/// default wrapper because their rasterized edges are either soft (shadow) or
/// small bitmaps where the ±1 pixel isn't visible (title bars).
#[derive(Debug)]
pub struct PixelSnapRescaleElement<E> {
    element: E,
    origin: Point<i32, Physical>,
    scale: Scale<f64>,
}

impl<E: Element> PixelSnapRescaleElement<E> {
    pub fn from_element(
        element: E,
        origin: Point<i32, Physical>,
        scale: impl Into<Scale<f64>>,
    ) -> Self {
        Self {
            element,
            origin,
            scale: scale.into(),
        }
    }
}

impl<E: Element> Element for PixelSnapRescaleElement<E> {
    fn id(&self) -> &smithay::backend::renderer::element::Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.element.current_commit()
    }

    fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> {
        self.element.src()
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let mut geo = self.element.geometry(scale);
        geo.loc -= self.origin;
        let mut out = corner_round_rect(geo.to_f64(), self.scale);
        out.loc += self.origin;
        out
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        // Conservative damage: over-expand rather than under-expand so repaints
        // never miss pixels. Matches smithay's RescaleRenderElement behavior.
        self.element
            .damage_since(scale, commit)
            .into_iter()
            .map(|rect| rect.to_f64().upscale(self.scale).to_i32_up())
            .collect()
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        // Opaque regions must be conservative in the OTHER direction: never
        // claim a pixel is opaque unless it fully is. Shrink inward so the
        // fringe isn't mistakenly marked opaque.
        self.element
            .opaque_regions(scale)
            .into_iter()
            .map(|rect| {
                let x0 = ((rect.loc.x as f64) * self.scale.x).ceil() as i32;
                let y0 = ((rect.loc.y as f64) * self.scale.y).ceil() as i32;
                let x1 = (((rect.loc.x + rect.size.w) as f64) * self.scale.x).floor() as i32;
                let y1 = (((rect.loc.y + rect.size.h) as f64) * self.scale.y).floor() as i32;
                Rectangle::new(
                    Point::from((x0, y0)),
                    Size::from(((x1 - x0).max(0), (y1 - y0).max(0))),
                )
            })
            .collect()
    }

    fn alpha(&self) -> f32 {
        self.element.alpha()
    }

    fn kind(&self) -> Kind {
        self.element.kind()
    }

    fn is_framebuffer_effect(&self) -> bool {
        self.element.is_framebuffer_effect()
    }
}

impl<E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for PixelSnapRescaleElement<E> {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        self.element.draw(frame, src, dst, damage, opaque_regions, cache)
    }

    #[inline]
    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, Physical>,
        cache: &smithay::utils::user_data::UserDataMap,
    ) -> Result<(), GlesError> {
        self.element.capture_framebuffer(frame, src, dst, cache)
    }
}

render_elements! {
    pub OutputRenderElements<=GlesRenderer>;
    Background=RescaleRenderElement<PixelShaderElement>,
    TileBg=RescaleRenderElement<TileShaderElement>,
    Decoration=PixelSnapRescaleElement<MemoryRenderBufferRenderElement<GlesRenderer>>,
    Window=PixelSnapRescaleElement<WaylandSurfaceRenderElement<GlesRenderer>>,
    CsdWindow=PixelSnapRescaleElement<RoundedCornerElement>,
    Layer=WaylandSurfaceRenderElement<GlesRenderer>,
    Cursor=MemoryRenderBufferRenderElement<GlesRenderer>,
    CursorSurface=smithay::backend::renderer::element::Wrap<WaylandSurfaceRenderElement<GlesRenderer>>,
    Blur=TextureRenderElement<GlesTexture>,
}

// Shadow and Decoration share inner types with Background and Tile respectively.
// We can't add them to render_elements! because it generates conflicting From impls.
// Instead we construct them directly using the existing Background/Tile variants.
// Helpers below create the elements and wrap them in the correct variant.

/// Wrapper element that applies a rounded-corner clipping shader to a window's root surface.
pub struct RoundedCornerElement {
    inner: WaylandSurfaceRenderElement<GlesRenderer>,
    shader: GlesTexProgram,
    uniforms: Vec<Uniform<'static>>,
    corner_radius: f64,
    clip_top: bool,
}

impl RoundedCornerElement {
    pub fn new(
        inner: WaylandSurfaceRenderElement<GlesRenderer>,
        shader: GlesTexProgram,
        uniforms: Vec<Uniform<'static>>,
        corner_radius: f64,
        clip_top: bool,
    ) -> Self {
        Self { inner, shader, uniforms, corner_radius, clip_top }
    }
}

impl Element for RoundedCornerElement {
    fn id(&self) -> &smithay::backend::renderer::element::Id { self.inner.id() }
    fn current_commit(&self) -> CommitCounter { self.inner.current_commit() }
    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> { self.inner.location(scale) }
    fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> { self.inner.src() }
    fn transform(&self) -> Transform { self.inner.transform() }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> { self.inner.geometry(scale) }
    fn damage_since(
        &self, scale: Scale<f64>, commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }
    fn opaque_regions(
        &self, scale: Scale<f64>,
    ) -> OpaqueRegions<i32, Physical> {
        let regions = self.inner.opaque_regions(scale);
        if regions.is_empty() || self.corner_radius <= 0.0 {
            return regions;
        }
        let geo = self.geometry(scale);
        // +1 to cover anti-aliased fringe from smoothstep
        let r = (self.corner_radius * scale.x).ceil() as i32 + 1;
        let (w, h) = (geo.size.w, geo.size.h);
        if w <= 2 * r || h <= 2 * r {
            return regions;
        }
        let mut corners = Vec::with_capacity(4);
        if self.clip_top {
            corners.push(Rectangle::new((0, 0).into(), (r, r).into()));
            corners.push(Rectangle::new((w - r, 0).into(), (r, r).into()));
        }
        corners.push(Rectangle::new((0, h - r).into(), (r, r).into()));
        corners.push(Rectangle::new((w - r, h - r).into(), (r, r).into()));
        let rects: Vec<_> = regions.into_iter().collect();
        Rectangle::subtract_rects_many_in_place(rects, corners).into_iter().collect()
    }
    fn alpha(&self) -> f32 { self.inner.alpha() }
    fn kind(&self) -> Kind { self.inner.kind() }
}

impl RenderElement<GlesRenderer> for RoundedCornerElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        _user_data: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.override_default_tex_program(self.shader.clone(), self.uniforms.clone());
        let result = self.inner.draw(frame, src, dst, damage, opaque_regions, _user_data);
        frame.clear_tex_program_override();
        result
    }

    fn underlying_storage(
        &self, renderer: &mut GlesRenderer,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }
}
