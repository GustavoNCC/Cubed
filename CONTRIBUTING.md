# Guía de contribución

Gracias por contribuir a Cubed.

## Flujo de trabajo

1. Crea una rama desde `main`: `feat/<fase>-<descripcion>` o `fix/<descripcion>`.
2. Mantén el proyecto **compilable y con tests en verde** en cada commit.
3. No mezcles fases del Roadmap en un mismo PR.
4. Actualiza la documentación y el `CHANGELOG.md` cuando corresponda.

## Convención de commits

Usamos **Conventional Commits**:

```
feat:     nueva funcionalidad
fix:      corrección de bug
docs:     documentación
refactor: refactor sin cambio de comportamiento
test:     pruebas
chore:    tareas de mantenimiento / build
```

Ejemplo: `feat(domain): añade entidad Server con validación de puerto`

## Estándares de código

### Rust
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```
- Sin `unwrap()` en rutas de producción salvo invariantes garantizadas.
- Errores tipados con `thiserror`.

### TypeScript / React
```bash
npm run lint
npm run format:check
```
- Componentes funcionales + hooks.
- La UI no ejecuta lógica de sistema: solo `invoke()` de comandos Tauri.

## Definición de "Hecho" (por fase)

- [ ] Compila (`cargo build` + `npm run build`).
- [ ] Lints y formato en verde.
- [ ] Tests básicos pasando.
- [ ] Documentación actualizada.
- [ ] `CHANGELOG.md` actualizado.
- [ ] Commit(s) con mensaje convencional.
