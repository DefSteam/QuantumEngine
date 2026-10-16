// Widget settings: the accent colour (header icon and selected buttons).

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    property int cfg_accentMode
    property color cfg_accentColor

    Kirigami.FormLayout {
        QQC2.RadioButton {
            id: violet
            Kirigami.FormData.label: i18n("Accent colour:")
            text: i18n("QuantumEngine violet")
            checked: cfg_accentMode === 0
            onToggled: if (checked) cfg_accentMode = 0
        }

        QQC2.RadioButton {
            text: i18n("Match the Plasma theme")
            checked: cfg_accentMode === 1
            onToggled: if (checked) cfg_accentMode = 1
        }

        RowLayout {
            QQC2.RadioButton {
                text: i18n("Custom:")
                checked: cfg_accentMode === 2
                onToggled: if (checked) cfg_accentMode = 2
            }

            QQC2.Button {
                id: swatchButton
                implicitWidth: Kirigami.Units.gridUnit * 3
                onClicked: colourDialog.open()
                contentItem: Rectangle {
                    radius: 4
                    color: cfg_accentColor
                    border.width: 1
                    border.color: Kirigami.Theme.disabledTextColor
                }
                Accessible.name: i18n("Choose a custom accent colour")
            }
        }
    }

    ColorDialog {
        id: colourDialog
        selectedColor: cfg_accentColor
        onAccepted: {
            cfg_accentColor = selectedColor
            cfg_accentMode = 2
        }
    }
}
