from typing import List, Dict, Optional
from dataclasses import dataclass, field
import pickle
from email_validator import validate_email, EmailNotValidError


class NotUniqueError(Exception):
    pass


@dataclass
class Contact:
    id: Optional[int] = None
    name: str = ""
    email: str = ""
    errors: List = field(default_factory=lambda: {"email": set()})

    def email_errors(self):
        msg = ", ".join(str(e) for e in self.errors["email"])
        return msg

    def validate_email(self, contacts) -> bool:
        ok = True
        try:
            email_info = validate_email(self.email)
        except EmailNotValidError as e:
            self.errors["email"].update({e})
            ok = False

        all_emails = [c.email for c in contacts]
        print(all_emails)
        if self.email in all_emails:
            self.errors["email"].update({NotUniqueError("this email is not new")})
            ok = False

        print("ok", ok)
        return ok

    def save(self, contacts) -> bool:
        ok = self.validate_email(contacts)
        if ok:
            self.id = len(contacts)
            contacts.contacts.append(self)
            contacts.write()
        return ok


class Contacts:
    def __init__(self, contacts):
        self.contacts = contacts

    def write(self):
        with open("c.pickle", mode="wb") as file:
            pickle.dump(tuple(self.contacts), file)

    @classmethod
    def load(cls):
        with open("c.pickle", mode="rb") as file:
            obj = pickle.load(file)
        new = cls(list(obj))
        print(new)
        return new

    def search(self, x):
        return [c for c in self if x in c.name]

    def __str__(self):
        return "\n".join(str(item) for item in self.contacts)

    def __iter__(self):
        return iter(self.contacts)

    def __len__(self):
        return len(self.contacts)


alice = Contact(0, "alice", "aws@mail.to")
bob = Contact(1, "bob", "bob@mail.to")
init_contacts = Contacts([alice, bob])
init_contacts.write()
