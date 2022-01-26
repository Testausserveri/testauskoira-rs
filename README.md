<p align="center">
<img src="https://i.imgur.com/dT8RLvv.png" height="150" alt="Testauskoira">
</p>

**Testauskoira-rs** on yleisbotti, jota käytetään erilaisissa [Testausserverin](https://testausserveri.fi) kylmää konetta vaativissa tehtävissä Discordin puolella. Botin tarkoituksena on tukea palvelimen toimintaa. Tämä README.md on varastettu suoraan originaalin testauskoiran repoista.

Botin toimintaa ja sen tietoturvallisuutta voi tutkia tässä repositoriossa, johon on sen lähdekoodi julkaistuna kokonaisuudessaan läpinäkyvyyttä varten.
Otamme mieluusti vastaan myös kaikenmuotoisia ominaisuuksia, jos haluat sellaisen koodata ja tehdä pr. Myös issuesissa voit antaa feature requestejä, joita ylläpitäjät tai muut vapaaehtoiset voivat toteuttaa.

Parhaiten apua bottiin saa discordista käyttäjältä DrVilepis#5329.

## Ominaisuudet ja tehtävät

### Viestistatistiikan kerääminen lukuina tietokantaan

Testauskoira tallentaa tietokantaan viestien määrä/päivä/käyttäjä-dataa. Dataa käytetään analytiikkaan, statistiikan esittämiseen julkisesti kokonaisuutena (viestejä koko palvelimella päivän aikana), tai tulevaisuudessa aktiivisten käyttäjien palkitsemiseen. Käyttäjien viestien sisältöjä ei tallenneta.

### Roolien myöntäminen jäsenille

Testausserverin tarpeiden mukaan Testauskoira toimii apulaisbottina, joka myöntää rooleja jäsenille erilaisten ehtojen täyttyessä. Esimerkiksi itsepalveluna pyytäessä tai jokaiselle jäsenelle palvelimelle liittyessä.

### GitHub-organisaatioon kutsuminen

Käyttäjät voivat kutsua itsensä Testausserverin GitHub-organisaatioon sisään itsepalveluna.

### Kielletyn sisällön moderoiminen

Botti poistaa kaikki kielletyt tekstinpätkät jotka löytyvät blacklist.txt tiedostosta tässä repositoriossa

### Miten tätä vehjettä ajetaan?

Tarvitset .env tiedoston joka sisältää kyseiset arvot:
```
DATABASE_URL=
DISCORD_TOKEN=
MOD_CHANNEL_ID=
APPLICATION_ID=
GUILD_ID=
AWARD_CHANNEL_ID=
NO_REPORTS_ROLE_ID=
AWARD_ROLE_ID=
SILENCED_ROLE_ID=
ADMIN_ROLE_ID=
```

Lisäksi sinun tulee ottaa käytöön [discordin developer consolesta](https://discord.com/developers) seuraavat INTENTit:
* Presence Intent
* Server Members Intent

### Automaattinen julkaiseminen

Repository on konfiguroitu automaattisesti julkaisemaan itsensä halutulle palvelimelle. Palvelin pitää tosin valmistella ensiksi kloonaamalla tämä repository sinne ja asettamalla tiedoston `.env`-arvot. Kun joku puskee uuden muutoksen myöhemmin `main`-haaraan ohjelma rakennetaan Githubin palvelimella, pusketaan heidän dockerkuva-arkistoon, ladataan arkistosta tuotantopalvelimelle ja käynnistetään. 

Automaattisen julkaisemisen toimiminen vaatii seuraavien salaisten arvojen asettaminen repositoryn (tapahtuu osoitteessa: [https://github.com/user/repository/settings/secrets/actions](https://github.com/user/repository/settings/secrets/actions). Arvojen tulee olla:

| Avain | Arvo |
| --- | ----- |
| SSH_DIR | Hakemisto, jossa kloonattu repo sijaitsee. Esim `/home/testauskoira/testauskoira-rs`   |
| SSH_IP | Tämän tulee olla IP-osoite tai palvelin, jolle halutaan julkaista ohjelma. |
| SSH_PRIVATE_KEY | **Yksityinen** SSH-avain, jolla voi tunnistautua palvelimelle. |
| SSH_USER | Käyttäjänimi, jolla yritetään kirjautua SSH:n yli. |
