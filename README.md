![banner](/assets/banner.png)

# Chimera - The Only Mock API you will need ⚡

[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/ams003010/chimera)](https://rust-reportcard.xuri.me/report/github.com/ams003010/chimera)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

Chimera is a blazing-fast, configurable JSON server built with Rust and Actix-web. It allows you to serve JSON files as APIs with sorting, pagination, simulated latency, and route-based retrieval. Ideal for prototyping, mock APIs, or rapid development.

Now with **automatic data generation and null value simulation**, Chimera helps you mock more realistic and dynamic API responses effortlessly.

### Perfect for:
 - Mock HTTP API for Frontend Development
 - Mock HTTP API for Mobile App Development
 - IoT Device Simulation
 - Prototyping for Microservices

### Future support for: 
 - Webhook Simulation (🪝)
 - GraphQL Mocking (⬢)
 - WebSocket Testing (🕸️)
 - gRPC Simulation (🌍)
 - MQTT Broker Simulation (🍔)

## 🐲 Features

- **Serve JSON as an API** – Load any JSON file and serve it as structured API endpoints.
- **CRUD Support** – GET, POST, DELETE Support on all routes.
- **Auto Data Generation** – Generate mock data automatically from schema-based definitions.
- **Null Value Simulation** – Add controlled nulls to fields for realistic data modeling.
- **Route-based Data Retrieval** – Fetch data by route and ID.
- **Sorting Support** – Sort entries dynamically based on attributes.
- **Pagination Support** – Limit the number of records per request.
- **Simulated Latency** – Mimic real-world API delays for better testing.
- **Ultra-Fast Performance** – Leveraging Rust and Actix-web for speed and efficiency.
- **Easy Configuration** – Set up ports, file paths, latency, sorting, and pagination via CLI.

## 🐲 Installation

### On Windows

On Powershell (Run as Administer)
```
Invoke-WebRequest -Uri "https://github.com/AMS003010/Chimera/releases/download/v0.5.0/chimera-windows.exe" -OutFile "chimera.exe"
```

On Powershell (non-privileged)
```
.\chimera.exe --path data.json
```

### On Linux and Mac

```
curl -sL $(curl -s https://api.github.com/repos/AMS003010/chimera/releases/latest | jq -r '.assets[] | select(.name | test("chimera.*")) | .browser_download_url') -o chimera
chmod +x chimera
./chimera --path data.json
```

## 🐲 Usage

### CLI Commands

Here's all the available CLI commands

`chimera.exe --path .\data.json`: Start the Chimera server with data from `data.json` at default port `8080`

`chimera.exe --path .\data.json --port 4000`: Start the Chimera server with data from `data.json` at port `4000`

`chimera.exe --path .\data.json --sort products desc id`: Start server with records in `/products` route sorted by `id` in `asc` order

`chimera.exe --path .\data.json --page 3`: Start server with the records paginated with a factor `3`

`chimera.exe --path .\data.json --latency 100`: Start server with a simulated latency of `100 ms`


| Flag             | Description                                      |
|-----------------|--------------------------------------------------|
| `--path <file>`  | Path to the JSON file (Required)               |
| `--port <port>`  | Specify the server port (Default: 8080)        |
| `--latency <ms>` | Simulated latency in milliseconds (Optional)   |
| `--sort <route> <asc / desc> <attribute>` | Sort route data dynamically |
| `--page <num>`   | Paginate GET responses (Default: 0 - No Limit) |
| `-X`/`--auto_generate_data` | Enable auto-generation from a schema JSON |

### API Endpoints

| Method   | Endpoint        | Description                      |
| -------- | --------------- | -------------------------------- |
| `GET`    | `/ping`         | Health check (`Pong 🏓`)         |
| `GET`    | `/{route}`      | Retrieve all data under a route  |
| `GET`    | `/{route}/{id}` | Retrieve a specific record by ID |
| `POST`   | `/{route}`      | Add a record under a route       |
| `DELETE` | `/{route}`      | Delete all records under a route |
| `DELETE` | `/{route}/{id}` | Delete a specific record by ID   |

## 🔧 Auto Data Generation

With the `-X` flag, Chimera can generate data on the fly using a schema JSON structure like:
```
{
    "routes": [
        {
            "path":"data",
            "no_of_entries": 2,
            "schema": {
                "mssg_id": "id",
                "created_on": "date",
                "mssg": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"products",
            "no_of_entries": 7,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean"
            },
            "null_percentage": 0
        }
    ]
}
```
Pass this JSON file as an argument to `--pass`. 
- `no_of_entries`: Number of mock entries to generate
- `schema`: Define fields and their data type (`name`, `id`, `date`, `lorem`, etc.)
- `null_percentage`: Percentage of fields and rows to be randomly set as `null`

## 📜 Example Data JSON File (`data.json`)

```json
{
    "data":[
        {
            "mssg_id":1,
            "created_on":"25-03-24",
            "mssg":"It is a long established fact that a reader will be distracted by the readable content of a page when looking at its layout. The point of using Lorem Ipsum is that it has a more-or-less normal distribution of letters, as opposed to using 'Content here, content here', making it look like readable English. Many desktop publishing packages and web page editors now use Lorem Ipsum as their default model text, and a search for"
        },
        {
            "mssg_id":2,
            "created_on":"02-11-24",
            "mssg":"years old. Richard McClintock, a Latin professor at Hampden-Sydney College in Virginia, looked up one of the more obscure Latin words, consectetur, fro"
        }
    ],
    "products": [
        {
            "id":8,
            "name": "ball"
        },
        {
            "id":4,
            "name": "table"
        },
        {
            "id":6,
            "name": "ball"
        },
        {
            "id":7,
            "name": "ball"
        }
    ]
}
```

## 📜 Example Schema JSON File (`schema.json`)

```json
{
    "routes": [
        {
            "path":"data",
            "no_of_entries": 2,
            "schema": {
                "mssg_id": "id",
                "created_on": "date",
                "mssg": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"products",
            "no_of_entries": 7,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean"
            },
            "null_percentage": 0
        }
    ]
}
```

## 🐲 Maintainers
This project is maintained by [@AMS003010](https://github.com/AMS003010).

## 🐲 License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
