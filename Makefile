PREFIX  ?= /usr
DESTDIR ?=

CARGO   ?= cargo
BIN      = target/release/proart-power

.PHONY: build install uninstall clean

build:
	$(CARGO) build --release

# Lay down files only. Live activation (services/udev/integration) is done by
# install.sh so packaging (DESTDIR) stays side-effect free.
install: build
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/proart-power
	install -Dm644 systemd/proart-power.service        $(DESTDIR)$(PREFIX)/lib/systemd/system/proart-power.service
	install -Dm644 systemd/proart-power-resume.service $(DESTDIR)$(PREFIX)/lib/systemd/system/proart-power-resume.service
	install -Dm644 udev/99-proart-power.rules          $(DESTDIR)$(PREFIX)/lib/udev/rules.d/99-proart-power.rules
	install -Dm644 config/proart-power.conf            $(DESTDIR)/etc/proart-power.conf

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/proart-power
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/system/proart-power.service
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/system/proart-power-resume.service
	rm -f $(DESTDIR)$(PREFIX)/lib/udev/rules.d/99-proart-power.rules

clean:
	$(CARGO) clean
