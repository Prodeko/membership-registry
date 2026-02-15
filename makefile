up:
	echo "Remember to setup your .env file!"
	docker compose up -d
	cd frontend; npm install
	cd backend; sqlx migrate run

devdata:
	cd backend; sqlx migrate run
	cd backend; pip3 install flask psycopg2 python-dotenv requests
	cd backend; python3 create_dev_data.py

e2e:
	cd e2e; npx playwright test