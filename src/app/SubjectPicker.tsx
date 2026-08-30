import { Card } from '@/shared/ui/Card'
import { Field } from '@/shared/ui/Field'
import { useUi, type Subject } from '@/shared/store/ui'

/**
 * Che materia si sta per allenare.
 *
 * Sta nella radice e non in una feature perche' e' l'unico pezzo di interfaccia che
 * deve conoscerle tutte, e la regola di dipendenza vieta a una feature di nominarne
 * un'altra. Le due schermate di scelta lo ricevono gia' fatto.
 */

const SUBJECTS: { value: Subject; label: string; caption: string }[] = [
  { value: 'kana', label: 'かな', caption: 'Kana' },
  { value: 'kanji', label: '漢字', caption: 'Kanji' },
]

export function SubjectPicker() {
  const { subject, setSubject } = useUi()

  return (
    <Field label="Subject">
      <div className="grid grid-cols-2 gap-2">
        {SUBJECTS.map((s) => (
          <Card
            key={s.value}
            pressed={subject === s.value}
            onClick={() => setSubject(s.value)}
          >
            <span className="font-jp text-2xl" lang="ja">
              {s.label}
            </span>
            <span className="text-muted text-xs">{s.caption}</span>
          </Card>
        ))}
      </div>
    </Field>
  )
}
