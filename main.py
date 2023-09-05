from dataclasses import dataclass
from flask import Flask, redirect, url_for, request, render_template


app = Flask(__name__)

route = app.route


@dataclass
class Contact:
    name: str
    email: str


class Contacts:
    alice = Contact("alice", "aws@mail.to")
    bob = Contact("bob", "bob@mail.to")

    all_items = [alice, bob]

    def __init__(self, contacts):
        self.items = contacts

    @classmethod
    def all(cls):
        return cls(cls.all_items)

    @classmethod
    def search(cls, x):
        return cls(c for c in cls.all_items if x in c.name)

    def __str__(self):
        return "<br>".join(str(item) for item in self.items)


@route("/")
def index():
    return redirect("/contacts")


@route("/contacts")
def contacts():
    search = request.args.get("q")
    if search is not None:
        print(search)
        contact_set = Contacts.search(search)
    else:
        contact_set = Contacts.all()
    return render_template("index.html", contacts=contact_set)


app.run(host="0.0.0.0", port=81)
