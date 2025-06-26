# Schedule voter

This is a tool to collect feedback about accepted/confirmed submissions for a conference. Based on the feedback, it's possible to build a better schedule with the most popular submissions at prime time and with less events running at the same time that are favoured by the same audience.

# Architecture

There is a backend in Rust (this repository) that stores everything in a Postgres database. Authentication is generally done via OpenID connect. There are two frontends written in React. One for the audience where they give feedback (client frontend) and an admin frontend where the conference organizers can access the feedback (admin frontend).

# Building instructions

## Manual build

You have to build the frontends first. Go to frontends/admin and frontends/client and run the following commands in each directory: `npm install; npm run build`. Then you can build the backend in the root directory by running `cargo build --release`.

## Docker build

You can also build the application as a Docker image by running `docker build -t voter .`.

You can also build the application using *docker-compose* by running `docker-compose build`.

# Running

We would suggest to run the application using *docker-compose*. Copy `.env.template` to `.env` and then adjust some values. In general you can leave most settings unchanged, but you may need to configure an OpenID provider (the callback url will be always `/admin/callback`) and you may need to adjust your external URL. When you want *https* then you should configure an incoming reverse proxy. You may need to change the exposed port number in the `docker-compose.yml` file and you should set the external URL under which users see the application. Then just run `docker-compose up -d` and it will start running. Users who login will be able to create new conferences. When all admin users have logged in, then set `NEW_USERS_CAN_CREATE=false`.