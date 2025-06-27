![banner](/assets/banner.png)

# Chimera - The Only Mock API you will ever need ⚡

<!-- [![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/ams003010/chimera)](https://rust-reportcard.xuri.me/report/github.com/ams003010/chimera) -->
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Version](https://img.shields.io/badge/version-0.6.0-blue.svg)

Chimera is a blazing-fast, configurable JSON server built with Axum. It allows you to serve JSON files as APIs with full CRUD support, sorting, pagination, simulated latency, and route-based retrieval. Ideal for prototyping, mock APIs, or rapid development.

Now with **automatic data generation**, **null value simulation**, **long path support**, **form submission**, and **CORS control**, Chimera helps you mock realistic and dynamic API responses effortlessly.

### Perfect for:

* Mock HTTP API for Frontend Development
* Mock HTTP API for Mobile App Development
* IoT Device Simulation
* Prototyping for Microservices

### Future support for:

* Webhook Simulation (🪝)
* GraphQL Mocking (⬢)
* WebSocket Testing (🕸️)
* gRPC Simulation (🌍)
* MQTT Broker Simulation (🍔)

## 🐲 Features

* **Serve JSON as an API** – Load any JSON file and serve it as structured API endpoints.
* **Full CRUD Support** – GET, POST, DELETE, PATCH, PUT supported on all routes.
* **Support for Nested Routes** – Long paths like `/api/v2/data` are supported.
* **Auto Data Generation** – Generate mock data automatically from schema-based definitions.
* **Null Value Simulation** – Add controlled nulls to fields for realistic data modeling.
* **Route-based Data Retrieval** – Fetch data by route and ID.
* **Sorting Support** – Sort entries dynamically based on attributes.
* **Pagination Support** – Limit the number of records per request.
* **Simulated Latency** – Mimic real-world API delays for better testing.
* **Ultra-Fast Performance** – Leveraging Rust and Axum for speed and efficiency.
* **Easy Configuration** – Set up ports, file paths, latency, sorting, and pagination via CLI.
* **Form Submission** – Supports `POST` form submissions at `/submit-form`.
* **CORS Control** – Enable/disable CORS by specifying allowed domains in a `chimera.cors` file.

## 🐲 Installation

### On Windows
Download and run directly:
```powershell
# Download the latest release
Invoke-WebRequest -Uri "https://github.com/AMS003010/Chimera/releases/latest/download/chimera-windows.exe.zip" -OutFile "chimera-windows.zip"

# Extract the zip file
Expand-Archive -Path "chimera-windows.zip" -DestinationPath "."

# Rename the binary
Rename-Item chimera-windows.exe chimera-cli.exe

# Run chimera
.\chimera-cli.exe --path data.json
```

### On Linux

#### Using the precompiled binary:
```bash
# Download and extract the latest Linux binary
curl -sL https://github.com/AMS003010/Chimera/releases/latest/download/chimera-linux.zip -o chimera-linux.zip
unzip chimera-linux.zip
chmod +x chimera-linux
mv chimera-linux chimera-cli
./chimera-linux --path data.json
```

#### Using the Debian package (Ubuntu/Debian):
```bash
# Download the latest .deb package
wget https://github.com/AMS003010/Chimera/releases/latest/download/chimera_$(curl -s https://api.github.com/repos/AMS003010/Chimera/releases/latest | jq -r '.tag_name' | sed 's/^v//')_amd64.deb

# Install the package
sudo dpkg -i chimera-cli_*_amd64.deb

# Run chimera (now available system-wide)
chimera-cli --path data.json
```

### On macOS
```bash
# Download and extract the latest macOS binary
curl -sL https://github.com/AMS003010/Chimera/releases/latest/download/chimera-macos.zip -o chimera-macos.zip
unzip chimera-macos.zip
chmod +x chimera-macos
./chimera-macos --path data.json
```

### Build from Source
```bash
git clone https://github.com/AMS003010/Chimera.git
cd Chimera
cargo install --path .
chimera-cli --path data.json
```

### Quick Download Script (Linux/macOS)
```bash
# One-liner to download and set up the latest release for your platform
curl -s https://api.github.com/repos/AMS003010/Chimera/releases/latest | \
jq -r ".assets[] | select(.name | test(\"$(uname -s | tr '[:upper:]' '[:lower:]')\")) | .browser_download_url" | \
xargs curl -sL -o chimera.zip && unzip chimera.zip && chmod +x chimera-* && echo "Download complete!"
```

## 🐲 Usage

### CLI Commands

Here's all the available CLI commands

`chimera-cli.exe --path .\data.json`: Start the Chimera server with data from `data.json` at default port `8080`

`chimera-cli.exe --path .\data.json --port 4000`: Start the Chimera server at port `4000`

`chimera-cli.exe --path .\data.json --sort products desc id`: Sort records in `/products` route by `id` in `desc` order

`chimera-cli.exe --path .\data.json --page 3`: Start server with the records paginated with a factor `3`

`chimera-cli.exe --path .\data.json --latency 100`: Simulate latency of `100 ms`

`chimera-cli.exe --path .\schema.json -X`: Enable automatic data generation using schema from `schema.json`

`chimera-cli.exe --path .\data.json --cors`: Enable CORS and allow only domains from `chimera.cors` file

> \[!NOTE]
> Use multiple arguments together for more diverse control

### CORS Configuration

To enable CORS, create a file named `chimera.cors` in the same directory as the binary with allowed domain(s):

```
http://localhost:3000
https://example.com
https://api.example.com
https://*.example.org
http://127.0.0.1:8080
```

### API Endpoints

| Method   | Endpoint        | Description                              |
| -------- | --------------- | ---------------------------------------- |
| `GET`    | `/`             | Health check                             |
| `GET`    | `/{route}`      | Retrieve all data under a route          |
| `GET`    | `/{route}/{id}` | Retrieve a specific record by ID         |
| `POST`   | `/{route}`      | Add a record under a route               |
| `DELETE` | `/{route}`      | Delete all records under a route         |
| `DELETE` | `/{route}/{id}` | Delete a specific record by ID           |
| `PUT`    | `/{route}/{id}` | Replace a specific record by ID          |
| `PATCH`  | `/{route}/{id}` | Partially update a specific record by ID |
| `POST`   | `/submit-form`  | Handle form submissions (URL-encoded)    |

## 🔧 Auto Data Generation

With the `-X` flag, Chimera can generate data on the fly using a schema JSON structure like:

```json
{
    "routes": [
        {
            "path":"api/v2/data",
            "no_of_entries": 2,
            "schema": {
                "id": "id",
                "created_on": "date",
                "mssg": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"products",
            "no_of_entries": 700,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean",
                "date": "datetime",
                "desc": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"api/products",
            "no_of_entries": 70,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean",
                "date": "datetime",
                "desc": "lorem"
            },
            "null_percentage": 0
        }
    ]
}
```

Pass this JSON file as an argument to `--path`

* `path`: Name of the route
* `no_of_entries`: Number of mock entries to generate
* `schema`: Define fields and their data type

  * `name`: Random name
  * `id`: Random number
  * `integer`: Random number
  * `date`: Date in `DD-MM-YYYY` format
  * `datetime`: Date in `DD-MM-YYYYTHH:MM:SS` format
  * `lorem`: Random text
  * `string`: Random word
  * `boolean`: Random boolean value
* `null_percentage`: Percentage of fields and rows to be randomly set as `null`

## 📜 Example Data JSON File (`data.json`)

```json
{
    "data":[
        {
            "id":1,
            "created_on":"25-03-24",
            "mssg":"Why spiders? Why couldn’t it be ‘follow the butterflies’?"
        },
        {
            "id":2,
            "created_on":"02-11-24",
            "mssg":"He can run faster than Severus Snape confronted with shampoo."
        }
    ],
    "api/products": [
        {
            "id":80,
            "name": "veritaserum"
        },
        {
            "id":40,
            "name": "polyjuice potion"
        },
        {
            "id":60,
            "name": "felix felicis"
        }
    ]
}
```

## 📜 Example Schema JSON File (`schema.json`)

```json
{
    "routes": [
        {
            "path":"api/v2/data",
            "no_of_entries": 2,
            "schema": {
                "id": "id",
                "created_on": "date",
                "mssg": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"products",
            "no_of_entries": 700,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean",
                "date": "datetime",
                "desc": "lorem"
            },
            "null_percentage": 0
        },
        {
            "path":"api/products",
            "no_of_entries": 70,
            "schema": {
                "id": "id",
                "rsnd": "integer",
                "name": "name",
                "probability": "boolean",
                "date": "datetime",
                "desc": "lorem"
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
