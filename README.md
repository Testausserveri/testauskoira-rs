<p align="center">
<img src="https://i.imgur.com/dT8RLvv.png" height="150" alt="Testauskoira">
</p>

**Testauskoira-rs** on yleisbotti, jota käytetään erilaisissa [Testausserverin](https://testausserveri.fi) kylmää konetta vaativissa tehtävissä Discordin puolella. Botin tarkoituksena on tukea palvelimen toimintaa. Tämä README.md on varastettu suoraan originaalin testauskoiran repoista.

Botin toimintaa ja sen tietoturvallisuutta voi tutkia tässä repositoriossa, johon on sen lähdekoodi julkaistuna kokonaisuudessaan läpinäkyvyyttä varten.
Otamme mieluusti vastaan myös kaikenmuotoisia ominauisuuksia, jos haluat sellaisen koodata ja tehdä pr. Myös issuesissa voit antaa feature requestejä, joita ylläpitäjät tai muut vapaaehtoiset voivat toteuttaa.

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
