import QtQuick 2.15
import QtQuick.Window 2.15
import Qt.labs.qmlmodels 1.0
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.11

import HostTableModel 1.0

import "./Dialog"
import "./Button"
import "./DetailsView"
import "./Misc"
import "js/Utils.js" as Utils

ApplicationWindow {
    id: root
    visible: true
    minimumWidth: 1400
    minimumHeight: 800
    width: minimumWidth + 100
    height: minimumHeight

    onWidthChanged: {
        hostTable.forceLayout()
    }

    property var _detailsDialogs: {}
    property int _textDialogPendingInvocation: 0


    menuBar: ToolBar {
        background: BorderRectangle {
            backgroundColor: Theme.backgroundColor
            borderColor: Theme.borderColor
            borderBottom: 1
        }

        RowLayout {
            anchors.fill: parent

            ToolButton {
                icon.source: "qrc:/main/images/button/add"
                onClicked: {
                    hostConfigurationDialog.hostId = ""
                    hostConfigurationDialog.open()
                }
            }

            ToolButton {
                enabled: _hostTableModel.selectedRow >= 0
                opacity: Theme.opacity(enabled)
                icon.source: "qrc:/main/images/button/entry-edit"
                onClicked: {
                    ConfigManager.begin_host_configuration()
                    hostConfigurationDialog.hostId = _hostTableModel.getSelectedHostId()
                    hostConfigurationDialog.open()
                }
            }

            ToolButton {
                enabled: _hostTableModel.selectedRow >= 0
                opacity: Theme.opacity(enabled)
                icon.source: "qrc:/main/images/button/remove"
                onClicked: {
                    ConfigManager.begin_host_configuration()
                    ConfigManager.removeHost(_hostTableModel.getSelectedHostId())
                    ConfigManager.end_host_configuration()
                    reloadConfiguration()
                }
            }

            ToolSeparator {
            }


            /* TODO: implement later?
            Row {
                id: searchRow
                spacing: Theme.spacing_loose()

                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing_loose() * 4

                Label {
                    text: "Search:"
                    anchors.verticalCenter: parent.verticalCenter
                }

                TextField {
                    id: searchInput
                    text: "Search by name or address"

                    width: parent.width * 0.5
                }
            }
            */

            // Spacer
            Item {
                Layout.fillWidth: true
            }

            ToolSeparator {
            }

            ToolButton {
                icon.source: "qrc:/main/images/button/configure"
                onClicked: {
                    preferencesDialog.open()
                }
            }
        }
    }

    Connections {
        target: HostDataManager

        function onUpdate_received(hostId) {
            _hostTableModel.dataChangedForHost(hostId)

            if (hostId === detailsView.hostId) {
                detailsView.refresh()
            }

            _hostTableModel.displayData = HostDataManager.getDisplayData()
        }

        function onHost_initialized(hostId) {
            let categories = CommandHandler.get_all_host_categories(hostId)
            for (const category of categories) {
                let invocation_ids = CommandHandler.refresh_monitors_of_category(hostId, category)
                HostDataManager.add_pending_monitor_invocations(hostId, category, invocation_ids)
            }
        }

        function onHost_initialized_from_cache(hostId) {
            let categories = CommandHandler.get_all_host_categories(hostId)
            for (const category of categories) {
                let invocation_ids = CommandHandler.cached_refresh_monitors_of_category(hostId, category)
                HostDataManager.add_pending_monitor_invocations(hostId, category, invocation_ids)
            }
        }

        function onMonitor_state_changed(hostId, monitorId, newCriticality) {
            hostTable.highlightMonitor(hostId, monitorId, newCriticality)
        }

        function onCommand_result_received(commandResultJson) {
            let commandResult = JSON.parse(commandResultJson)

            if (commandResult.show_in_notification === true &&
                (commandResult.criticality !== "Normal" || Theme.hide_info_notifications() === false)) {

                if (commandResult.error !== "") {
                    snackbarContainer.addSnackbar(commandResult.criticality, commandResult.error)
                }
                else {
                    snackbarContainer.addSnackbar(commandResult.criticality, commandResult.message)
                }
            }

            let dialogInstanceId = _detailsDialogs[commandResult.invocation_id]
            if (typeof dialogInstanceId !== "undefined") {
                let dialog = detailsDialogManager.get(dialogInstanceId)
                dialog.text = commandResult.message
                dialog.errorText = commandResult.error
                dialog.criticality = commandResult.criticality
            }
            else if (_textDialogPendingInvocation === commandResult.invocation_id) {
                textDialog.text = commandResult.message
            }
        }

        function onError_received(criticality, message) {
            if (criticality === "Critical") {
                // TODO: something better. This is not really an alert dialog.
                textDialog.text = message
                textDialog.open()
            }
            else {
                snackbarContainer.addSnackbar(criticality, message)
            }
        }
    }

    Connections {
        target: CommandHandler

        // Set up confirmation dialog on signal.
        function onConfirmation_dialog_opened(text, hostId, commandId, commandParams) {
            confirmationDialogLoader.setSource("./Dialog/ConfirmationDialog.qml", { text: text }) 
            confirmationDialogLoader.item.onAccepted.connect(() => CommandHandler.execute_confirmed(hostId, commandId, commandParams))
        }

        function onDetails_dialog_opened(invocationId) {
            let instanceId = detailsDialogManager.create()
            _detailsDialogs[invocationId] = instanceId
        }

        function onText_dialog_opened(invocationId) {
            _textDialogPendingInvocation = invocationId
            textDialog.open()
        }

        function onInput_dialog_opened(input_specs_json, hostId, commandId, commandParams) {
            let inputSpecs = JSON.parse(input_specs_json)

            inputDialog.inputSpecs = inputSpecs
            // TODO: need to clear previous connections?
            inputDialog.onInputValuesGiven.connect((inputValues) => {
                CommandHandler.execute_confirmed(hostId, commandId, commandParams.concat(inputValues))
            })
            inputDialog.open()
        }
    }

    Connections {
        target: _hostTableModel

        function onSelectedRowChanged() {
            detailsView.hostId = _hostTableModel.getSelectedHostId()

            if (detailsView.hostId !== "") {
                if (!HostDataManager.is_host_initialized(detailsView.hostId)) {
                    CommandHandler.initialize_host(detailsView.hostId)
                }
            }
        }

        function onSelectionActivated() {
            body.splitSize = 0.8
        }

        function onSelectionDeactivated() {
            body.splitSize = 0.0
        }
    }

    Connections {
        target: DesktopPortal
        function onOpenFileResponse(token) {
            console.log("************ Received open file response: " + token)
            // TODO
        }

        function onError(message) {
            snackbarContainer.addSnackbar("Critical", message)
        }
    }

    Component.onCompleted: {
        _detailsDialogs = {}

        // Starts the thread that receives host state updates in the backend.
        HostDataManager.receive_updates()
        // Starts the thread that receives portal responses from D-Bus.
        DesktopPortal.receiveResponses()

        console.log("Current color palette: ", palette)

        if (HostDataManager.refresh_hosts_on_start()) {
            CommandHandler.force_initialize_hosts()
        }
    }

    onClosing: {
        CommandHandler.stop()
        HostDataManager.stop()
        DesktopPortal.stop()
    }

    Item {
        id: body
        anchors.fill: parent
        property real splitSize: 0.0

        SplitView {
            anchors.fill: parent
            orientation: Qt.Vertical

            HostTable {
                id: hostTable
                width: parent.width
                SplitView.fillHeight: true

                model: HostTableModel {
                    id: _hostTableModel
                    selectedRow: -1
                    displayData: HostDataManager.getDisplayData()
                }
            }

            HostDetails {
                id: detailsView
                visible: body.splitSize > 0.01
                width: parent.width
                hostId: _hostTableModel.getSelectedHostId()

                SplitView.minimumHeight: 0.5 * body.splitSize * body.height
                SplitView.preferredHeight: body.splitSize * body.height
                SplitView.maximumHeight: 1.5 * body.splitSize * body.height

                onMinimizeClicked: {
                    body.splitSize = 0.8
                }
                onMaximizeClicked: {
                    body.splitSize = 1.0
                }
                onCloseClicked: {
                    _hostTableModel.toggleRow(_hostTableModel.selectedRow)
                }
            }
        }

        // Animations
        Behavior on splitSize {
            NumberAnimation {
                duration: Theme.animation_duration()
                easing.type: Easing.OutQuad

                onFinished: {
                    // TODO: animate?
                    hostTable.centerRow()
                }
            }
        }
    }

    // Dynamic component loaders
    Loader {
        id: confirmationDialogLoader
    }

    DynamicObjectManager {
        id: detailsDialogManager

        DetailsDialog {
            y: root.y + 50
            x: root.x + 50
            width: root.width
            height: root.height
        }
    }

    SnackbarContainer {
        id: snackbarContainer
        anchors.fill: parent
        anchors.margins: 20
    }


    // Modal dialogs
    InputDialog {
        id: inputDialog
        visible: false
        anchors.centerIn: parent
    }

    HostConfigurationDialog {
        id: hostConfigurationDialog
        visible: false
        anchors.centerIn: parent
        bottomMargin: 0.12 * parent.height

        onConfigurationChanged: {
            reloadConfiguration()
        }
    }

    PreferencesDialog {
        id: preferencesDialog
        visible: false
        anchors.centerIn: parent
        bottomMargin: 0.12 * parent.height

        onConfigurationChanged: {
            reloadConfiguration()
        }
    }

    TextDialog {
        id: textDialog
        visible: false
        anchors.centerIn: parent
        width: Utils.clamp(implicitWidth, root.width * 0.5, root.width * 0.8)
        height: Utils.clamp(implicitHeight, root.height * 0.5, root.height * 0.8)
    }

    function reloadConfiguration() {
        HostDataManager.reset()
        _hostTableModel.toggleRow(_hostTableModel.selectedRow)
        _hostTableModel.displayData = HostDataManager.getDisplayData()

        let configs = ConfigManager.reloadConfiguration()
        CommandHandler.reconfigure(configs[0], configs[1])
    }
}