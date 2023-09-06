from flask import (
    Flask,
    redirect,
    url_for,
    request,
    render_template,
    flash,
)
from model import Contacts, Contact


app = Flask(__name__)
app.secret_key = "dev"


route = app.route


@route("/")
def index():
    return redirect("/contacts")


@route("/contacts")
def contacts():
    search = request.args.get("q")
    contacts = Contacts.load()
    if search is not None:
        contact_set = contacts.search(search)
    else:
        contact_set = contacts
    return render_template("index.html", contacts=contact_set)


@route("/contacts/new", methods=["GET"])
def contacts_new_get():
    return render_template("new.html", contact=Contact())


@route("/contacts/new", methods=["POST"])
def contacts_save():
    c = Contact(None, request.form["name"], request.form["email"])
    contacts = Contacts.load()
    if c.save(contacts):
        flash("Succes!")
        return redirect("/contacts")

    return render_template("new.html", contact=c)


app.run(host="0.0.0.0", port=81)
