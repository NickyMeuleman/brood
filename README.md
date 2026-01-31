To investigate:

How to send Rust thiserror annotations to javascript?
Now I duplicate these strings, once in #[error("")] on the backend and once in lib/errors.ts on the frontend

Validation without duplication:
Backend validation with the rust validator crate.
Frontend validation with the zod package.
Do I have to duplicate the logic or can I write it once and reuse and extend it?