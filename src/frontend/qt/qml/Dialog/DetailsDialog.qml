import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Window 2.15
// TODO: Get rid of Material since it mostly just causes issues with correct background colors.
import QtQuick.Controls.Material 2.15

import ".."


Window {
    property var identifier: ""
    property var text: ""
    property var errorText: ""
    property var criticality: ""

    id: root
    visible: true
    color: Material.background

    Material.theme: Material.Dark

    Dialog {
        modal: false
        standardButtons: Dialog.Ok
        implicitHeight: root.height
        implicitWidth: root.width
        Component.onCompleted: visible = true

        onAccepted: root.close()

        WorkingSprite {
            visible: root.text === "" && root.errorText === ""
        }

        ScrollView {
            visible: root.text !== ""
            anchors.fill: parent

            JsonTextFormat {
                jsonText: root.text
            }
        }

        AlertMessage {
            id: textContent
            text: root.errorText
            criticality: root.criticality
            visible: root.errorText !== ""
        }
    }
}