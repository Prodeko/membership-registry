up:
	docker compose up -d
	rm -f backend/.env
	cp backend/.env.template backend/.env
	echo "" >> backend/.env
	echo "" >> backend/.env
	echo "$$(bash ../auth/scripts/create-oauth-client.sh -n membership-registry --format dotenv)" >> backend/.env
	cd frontend; npm install
	cd backend; sqlx migrate run

devdata:
	cd backend; sqlx migrate run
	cd backend; pip3 install flask psycopg2 python-dotenv requests
	cd backend; python3 create_dev_data.py