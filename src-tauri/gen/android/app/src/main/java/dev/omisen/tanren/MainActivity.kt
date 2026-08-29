package dev.omisen.tanren

import android.graphics.Color
import android.os.Bundle
import android.view.ViewGroup
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Icone chiare su entrambe le barre, sempre. L'app ha un solo fondo, l'`ink`
    // scuro della palette, quindi seguire il tema chiaro/scuro del sistema
    // produrrebbe icone nere su fondo nero meta' delle volte.
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
    )
    super.onCreate(savedInstanceState)
  }

  /**
   * Tiene il contenuto dentro l'area utile: fuori dalle barre di sistema, dal
   * ritaglio del display e dalla tastiera.
   *
   * # Perche' lo fa il nativo e non il CSS
   *
   * Le `env(safe-area-inset-*)` sembrano la risposta ovvia, ma su Android coprono il
   * **ritaglio del display**, non le barre di sistema: fidarsi di quelle lasciava il
   * contenuto sotto la status bar e sotto la navigation bar. Gli insets devono avere
   * un padrone solo, e ora e' questo file: il lato web non li tocca piu'.
   *
   * Per la tastiera non c'era comunque scelta. Da Android 15, sulle app con
   * `targetSdk` 35 o piu' (il nostro e' 36), il bordo a bordo e' imposto e
   * `windowSoftInputMode="adjustResize"` non ha piu' alcun effetto: gli insets sono
   * responsabilita' dell'app.
   *
   * # Come
   *
   * Si agisce sui **margini**, non sui padding: la WebView si rimpicciolisce davvero,
   * quindi la pagina si ridispone invece di scorrere sotto le barre, e dietro ai
   * margini si vede `@color/ink`, che e' lo stesso fondo della pagina. Il risultato a
   * schermo e' un colore pieno da bordo a bordo con il contenuto al sicuro dentro.
   *
   * In fondo vince il piu' alto fra tastiera e navigation bar: quando l'IME e' aperto
   * copre gia' la barra, e sommarli lascerebbe una fascia vuota.
   *
   * Gli insets vengono **consumati**, perche' qui sono gestiti per intero. Cosi' la
   * WebView non li applica una seconda volta per conto suo.
   *
   * `onWebViewCreate` e' il punto di estensione che `WryActivity` espone: la WebView
   * la crea wry, non c'e' un layout XML in cui agganciarsi.
   */
  override fun onWebViewCreate(webView: WebView) {
    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      val bottom = maxOf(keyboard, bars.bottom)

      val params = view.layoutParams
      if (params is ViewGroup.MarginLayoutParams &&
        (params.leftMargin != bars.left || params.topMargin != bars.top ||
          params.rightMargin != bars.right || params.bottomMargin != bottom)
      ) {
        params.leftMargin = bars.left
        params.topMargin = bars.top
        params.rightMargin = bars.right
        params.bottomMargin = bottom
        view.layoutParams = params
      }

      WindowInsetsCompat.CONSUMED
    }
  }
}
