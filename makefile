up:
	rm backend/.env
	cp backend/.env.template backend/.env
	echo "" >> backend/.env
	echo "" >> backend/.env
	echo "$$(bash ../scripts/create-oauth-client.sh -n membership-registry --format dotenv)" >> backend/.env
	cd frontend; npm install
	cd backend; sqlx migrate run

devdata:
	cd backend; sqlx migrate run
	cd backend; pip install flask psycopg2 python-dotenv requests
	cd backend; python create_dev_data.py