import QtQuick 2.15
import QtQuick.Controls 2.15
import Qt.labs.qmlmodels 1.0
import QtGraphicalEffects 1.15

import "../Button"
import "../Text"

Item {
    id: root
    property string text: ""
    // TODO: don't hardcode
    property string color: "#444444"
    property bool showRefreshButton: false
    property bool showMinimizeButton: false
    property bool showMaximizeButton: false
    property bool showCloseButton: false
    property bool showOpenInWindowButton: false
    property bool showSaveButton: false

    property bool _maximized: false

    implicitWidth: parent.width
    implicitHeight: 30

    signal refreshClicked()
    signal openInWindowClicked()
    signal maximizeClicked()
    signal minimizeClicked()
    signal closeClicked()
    signal saveClicked()

    Rectangle {
        color: root.color
        anchors.fill: parent

        NormalText {
            anchors.verticalCenter: parent.verticalCenter
            leftPadding: 10
            text: root.text
            font.pointSize: 12
        }
    }

    Row {
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        anchors.rightMargin: 5
        spacing: 5

        RefreshButton {
            size: 0.9 * parent.height
            anchors.verticalCenter: parent.verticalCenter
            onClicked: root.refreshClicked()
            visible: root.showRefreshButton
        }

        ImageButton {
            size: 0.9 * parent.height
            anchors.rightMargin: 5
            anchors.verticalCenter: parent.verticalCenter
            imageSource: "qrc:/main/images/button/window-new"
            flatButton: true
            tooltip: "Open in new window"
            onClicked: root.openInWindowClicked()
            visible: root.showOpenInWindowButton
        }

        ImageButton {
            size: 0.9 * parent.height
            anchors.verticalCenter: parent.verticalCenter
            imageSource: "qrc:/main/images/button/document-save"
            flatButton: true
            tooltip: "Save"
            onClicked: root.saveClicked()
            visible: root.showSaveButton
        }

        ImageButton {
            size: 0.9 * parent.height
            anchors.verticalCenter: parent.verticalCenter
            imageSource: "qrc:/main/images/button/maximize"
            flatButton: true
            tooltip: "Maximize"
            onClicked: {
                root.maximizeClicked()
                root._maximized = true
            }
            visible: root.showMaximizeButton && !root._maximized
        }

        ImageButton {
            size: 0.9 * parent.height
            anchors.verticalCenter: parent.verticalCenter
            imageSource: "qrc:/main/images/button/minimize"
            flatButton: true
            tooltip: "Minimize"
            onClicked: {
                root.minimizeClicked()
                root._maximized = false
            }
            visible: root.showMinimizeButton && root._maximized
        }

        ImageButton {
            size: 0.9 * parent.height
            anchors.verticalCenter: parent.verticalCenter
            imageSource: "qrc:/main/images/button/close"
            // By default this icon is black, so changing it here.
            color: Theme.iconColor
            imageRelativeWidth: 0.5
            imageRelativeHeight: 0.8
            flatButton: true
            tooltip: "Close"
            onClicked: root.closeClicked()
            visible: root.showCloseButton
        }
    }
}