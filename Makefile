run:
	@cargo run inscription 

fix/path:
	@sed -Ei '' 's|(include_str!)\(.+\)|\1\("\.\./prisma/schema\.prisma"\)|g' src/prisma.rs

db/generate:
	@cargo prisma generate

db/migrate:
	@cargo prisma migrate dev
	@make fix/path