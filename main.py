from flask import (
    Flask,
    redirect,
    url_for,
    request,
    render_template,
    flash,
    abort,
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


@route("/contacts/<contact_id>")
def contacts_view(contact_id=0):
    contacts = Contacts.load()
    contact = contacts.find(contact_id)
    if contact is None:
        abort(404)

    return render_template("view.html", contact=contact)


@route("/contacts/new", methods=["GET"])
def contacts_new_get():
    return render_template("new.html", contact=Contact())


@route("/contacts/<id>/edit", methods=["GET"])
def edit_get(id=0):
    contacts = Contacts.load()
    contact = contacts.find(int(id))
    return render_template("edit.html", contact=contact)


@route("/contacts/<id>/edit", methods=["POST"])
def edit_post(id=0):
    contacts = Contacts.load()
    contact = contacts.find(int(id))
    contact.name = request.form["name"]
    contact.email = request.form["email"]
    ok = contact.validate_email(contacts)
    if not ok:
        return render_template("edit.html", contact=contact)
    contacts.write()
    flash("Success!")
    return redirect("/contacts/" + str(id))


@route("/contacts/<id>/delete", methods=["GET"])
def get_delete(id=0):
    contacts = Contacts.load()
    contact = contacts.find(int(id))
    return render_template("delete.html", contact=contact)


@route("/contacts/<id>/delete", methods=["POST"])
def delete_contact(id=0):
    contacts = Contacts.load()
    deleted = contacts.pop(id)
    contacts.write()
    flash(f"Deleted Contact{deleted.name}")
    return redirect("/contacts")


@route("/contacts/new", methods=["POST"])
def contacts_save():
    c = Contact(None, request.form["name"], request.form["email"])
    contacts = Contacts.load()
    if c.save(contacts):
        flash("Succes!")
        return redirect("/contacts")

    return render_template("new.html", contact=c)


app.run(host="0.0.0.0", port=81)
