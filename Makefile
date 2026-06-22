PREFIX  ?= /usr
DESTDIR ?=

CARGO   ?= cargo
BIN      = target/release/ergctl

.PHONY: build install uninstall clean

build:
	$(CARGO) build --release

# Lay down files only. Live activation (services/udev/integration) is done by
# install.sh so packaging (DESTDIR) stays side-effect free.
install: build
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/ergctl
	install -Dm644 systemd/ergctl.service        $(DESTDIR)$(PREFIX)/lib/systemd/system/ergctl.service
	install -Dm644 systemd/ergctl-resume.service $(DESTDIR)$(PREFIX)/lib/systemd/system/ergctl-resume.service
	install -Dm644 udev/99-ergctl.rules          $(DESTDIR)$(PREFIX)/lib/udev/rules.d/99-ergctl.rules
	install -Dm644 config/ergctl.conf            $(DESTDIR)/etc/ergctl.conf

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/ergctl
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/system/ergctl.service
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/system/ergctl-resume.service
	rm -f $(DESTDIR)$(PREFIX)/lib/udev/rules.d/99-ergctl.rules

clean:
	$(CARGO) clean
