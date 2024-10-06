up:
	rm backend/.env
	cp backend/.env.template backend/.env
	echo "" >> backend/.env
	echo "" >> backend/.env
	echo "$$(bash ../scripts/create-oauth-client.sh -n membership-registry --format dotenv)" >> backend/.env
	cd frontend; npm install
	cd backend; sqlx migrate run
	cd backend; cargo run -- generate --amount 50