/**
 * Il marchio accanto al wordmark, in un cerchio alto quanto il testo.
 *
 * **Viene dall'icona dell'app**, ritagliata dal master a 1024 px, e non da una favicon:
 * quelle sono fatte per la linguetta del browser e la piu' grande che il progetto ha e'
 * 180 px, ma soprattutto sono un'altra cosa dal marchio dell'app, e nel giro di qualche
 * ritocco divergerebbero. Il file e' 128 px, che coprono il cerchietto da 20 px CSS
 * anche a densita' quadrupla.
 *
 * **Alto quanto il testo e non di piu'**: la misura e' l'interlinea di `text-sm`, cioe'
 * la riga su cui il wordmark poggia, quindi il marchio si allinea invece di dominare.
 *
 * Non ha testo alternativo ed e' nascosto alla lettura assistita: il nome dell'app e'
 * gia' li' accanto nel titolo, e ripeterlo direbbe «Tanren Tanren».
 */
export function LogoMark() {
  return (
    <img
      src="/logo-mark.png"
      alt=""
      aria-hidden="true"
      className="size-5 shrink-0 rounded-full object-cover"
    />
  )
}
