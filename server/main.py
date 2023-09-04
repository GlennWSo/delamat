from flask import Flask


app = Flask(__name__)

route = app.route


@route("/")
def index():
    return "Hello, World!"


app.run(host="0.0.0.0", port=81)
